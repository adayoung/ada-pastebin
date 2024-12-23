use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Form, Path, Query, State},
    http::header::{CACHE_CONTROL, LOCATION},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Json, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use axum_csrf::{CsrfConfig, CsrfLayer, CsrfToken};
use dashmap::{DashMap, DashSet};
use serde::Serialize;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tower_cookies::{CookieManagerLayer, Cookies, Key};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

mod api;
mod cloudflare;
mod config;
mod discord;
mod forms;
mod gdrive;
mod oauth;
mod paste;
mod recaptcha;
mod runtime;
mod s3;
mod session;
mod static_files;
mod templates;
mod utils;

#[tokio::main]
async fn main() {
    // Set up the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init(); // Initialize the subscriber

    // Connect to the database
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("DATABASE_URL environment variable not set");
        std::process::exit(1);
    });

    let db = match PgPool::connect(&db_url).await {
        Ok(pool) => pool,
        Err(err) => {
            error!("Failed to connect to database: {}", err);
            std::process::exit(1);
        }
    };

    let config = config::AppConfig::new();
    let cookie_key = Key::from(config.cookie_key.as_bytes());

    let shared_state = Arc::new(runtime::AppState {
        cloudflare_q: DashSet::new(),
        config,
        cookie_key,
        counter: DashMap::new(),
        db,
    });

    s3::init_s3_client(&shared_state).await;
    discord::init_discord_client(&shared_state);
    gdrive::init_drive_client(&shared_state);

    let timer_state = shared_state.clone();
    tokio::spawn(async move {
        tokio::join!(
            paste::update_views(&timer_state, true),
            cloudflare::cleanup_cache(&timer_state, true, true),
        );
    });

    tokio::spawn(api::reset_api_limiter());

    let shutdown_state = shared_state.clone();
    tokio::spawn(async move {
        runtime::shutdown_signal().await;
        info!("Shutting down...");
        tokio::join!(
            paste::update_views(&shutdown_state, false),
            cloudflare::cleanup_cache(&shutdown_state, false, true),
        );
        shutdown_state.db.close().await;
        std::process::exit(0);
    });

    let bind_addr = format!(
        "{}:{}",
        shared_state.config.bind_addr, shared_state.config.port
    );

    let csrf_key = axum_csrf::Key::from(shared_state.config.cookie_key.as_bytes());
    let csrf_config = CsrfConfig::new()
        .with_cookie_name(utils::get_cookie_name(&shared_state, "xsrf").as_str())
        .with_cookie_path("/pastebin/")
        .with_cookie_same_site(utils::get_cookie_samesite(&shared_state))
        .with_secure(shared_state.config.cookie_secure)
        .with_key(Some(csrf_key))
        .with_salt(shared_state.config.cookie_salt.clone())
        .with_lifetime(time::Duration::seconds(0));

    // build our application with routes
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/pastebin/") }))
        .route("/pastebin/", get(pastebin).post(newpaste))
        .route("/pastebin/api/v1/create", post(api::create))
        .route("/pastebin/api/v1/delete", delete(api::delete))
        .route("/pastebin/:paste_id", get(getpaste).post(delpaste))
        .route("/pastebin/auth/discord/start", get(discord::start))
        .route("/pastebin/auth/discord/finish", get(discord::finish))
        .route("/pastebin/auth/gdrive/start", get(gdrive::auth_start))
        .route("/pastebin/auth/gdrive/finish", get(gdrive::auth_finish))
        .route("/pastebin/auth/logout", post(logout))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024)) // 32MB is a lot of log!
        .layer(CsrfLayer::new(csrf_config))
        .route("/pastebin/about", get(about))
        .route("/pastebin/search/", get(search))
        .route("/pastebinc/:paste_id/content", get(getdrivecontent))
        .layer(CookieManagerLayer::new())
        .layer(middleware::from_fn(utils::extra_sugar))
        .layer(middleware::from_fn_with_state(
            shared_state.clone(),
            utils::csp,
        ))
        .layer(TraceLayer::new_for_http())
        .route("/static/*path", get(static_files::handler))
        .route("/robots.txt", get(robots))
        .fallback(notfound)
        .with_state(shared_state);

    // run it
    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[allow(dead_code)]
async fn index(State(state): State<Arc<runtime::AppState>>) -> templates::BaseTemplate {
    templates::BaseTemplate {
        static_domain: state.config.static_domain.clone(),
    }
}

async fn about(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> templates::AboutTemplate {
    let (user_id, _) = utils::get_user_id(&state, &cookies);
    templates::AboutTemplate {
        static_domain: state.config.static_domain.clone(),
        user_id,
    }
}

async fn pastebin(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
    token: CsrfToken,
) -> impl IntoResponse {
    let (user_id, _) = utils::get_user_id(&state, &cookies);
    let template = templates::PastebinTemplate {
        static_domain: state.config.static_domain.clone(),
        recaptcha_key: state.config.recaptcha_key.clone(),
        csrf_token: token.authenticity_token().unwrap(),
        user_id,
    };

    (token, template)
}

async fn newpaste(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    cookies: Cookies,
    token: CsrfToken,
    Form(payload): Form<forms::PasteForm>,
) -> impl IntoResponse {
    let (user_id, session_id) = utils::get_user_id(&state, &cookies);

    // Verify the CSRF token
    if token.verify(&payload.csrf_token).is_err() {
        return (StatusCode::FORBIDDEN, "CSRF token is not valid!").into_response();
    }

    // Verify the recaptcha response
    let score = recaptcha::verify(&state.config.recaptcha_secret, "paste", &payload.token)
        .await
        .unwrap_or_else(|err| {
            error!("Error verifying recaptcha: {}", err);
            0.0
        });

    let gdrive_token = gdrive::get_drive_token(&state, &cookies);
    if payload.destination == forms::ValidDestination::GDrive && gdrive_token.is_empty() {
        return (StatusCode::FORBIDDEN, "Google Drive not authorized!").into_response();
    }

    // Create the paste
    let paste_id = match paste::new_paste(&state, &payload, score, user_id, session_id, &gdrive_token).await {
        Ok(id) => id,
        Err(err) => {
            return err.into_response();
        }
    };

    // Update the session with the new paste_id
    session::update_session(&state, &cookies, &paste_id);

    // Check for the presence of the X-Requested-With header
    if headers.contains_key("X-Requested-With") {
        (StatusCode::OK, paste_id).into_response()
    } else {
        (
            StatusCode::SEE_OTHER,
            [(LOCATION, format!("/pastebin/{}", paste_id))],
            "",
        )
            .into_response()
    }
}

async fn getpaste(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
    token: CsrfToken,
    Path(paste_id): Path<String>,
) -> impl IntoResponse {
    let (user_id, _) = utils::get_user_id(&state, &cookies);

    let paste = match paste::Paste::get(&state.db, &paste_id).await {
        Ok(paste) => paste,
        Err(err) => {
            return err.into_response();
        }
    };

    let mut owned = session::is_paste_in_session(&state, &cookies, &paste_id);
    if user_id.is_some() && user_id == paste.user_id {
        owned = true;
    }
    let views = paste.get_views(&state);
    let template = templates::PasteTemplate {
        static_domain: state.config.static_domain.clone(),
        content_url: paste.get_content_url(&state.config.s3_bucket_url),
        csrf_token: token.authenticity_token().unwrap(),
        user_id,
        paste,
        views,
        owned,
    };

    (StatusCode::OK, template).into_response()
}

async fn delpaste(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    cookies: Cookies,
    token: CsrfToken,
    Path(paste_id): Path<String>,
    Form(payload): Form<forms::PasteDeleteForm>,
) -> impl IntoResponse {
    let (user_id, _) = utils::get_user_id(&state, &cookies);

    // Verify the CSRF token
    if token.verify(&payload.csrf_token).is_err() {
        return (StatusCode::FORBIDDEN, "CSRF token is not valid!").into_response();
    }

    let paste = match paste::Paste::get(&state.db, &paste_id).await {
        Ok(paste) => paste,
        Err(err) => {
            return err.into_response();
        }
    };

    let mut owned = session::is_paste_in_session(&state, &cookies, &paste_id);
    if user_id.is_some() && user_id == paste.user_id {
        owned = true;
    }
    if !owned {
        return (StatusCode::FORBIDDEN, "You don't own this paste!").into_response();
    }

    match paste.delete(&state).await {
        Ok(_) => {}
        Err(err) => {
            return err.into_response();
        }
    };

    // Check for the presence of the X-Requested-With header
    if headers.contains_key("X-Requested-With") {
        (StatusCode::OK, "/pastebin/").into_response()
    } else {
        (StatusCode::SEE_OTHER, [(LOCATION, "/pastebin/")], "").into_response()
    }
}

async fn getdrivecontent(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    Path(paste_id): Path<String>,
) -> impl IntoResponse {
    if !headers.contains_key("X-Requested-With") {
        return (
            StatusCode::TEMPORARY_REDIRECT,
            [(LOCATION, format!("/pastebin/{}", paste_id))],
            "",
        )
            .into_response();
    }

    let paste = match paste::Paste::get(&state.db, &paste_id).await {
        Ok(paste) => paste,
        Err(err) => {
            return err.into_response();
        }
    };

    if let Some(gdrivedl_url) = &paste.gdrivedl {
        let response = match reqwest::get(gdrivedl_url).await {
            Ok(response) => response,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)).into_response()
            }
        };

        if !response.status().is_success() {
            // Remove metadata if Google Drive returns a 404
            if response.status() == StatusCode::NOT_FOUND {
                match paste.delete(&state).await {
                    Ok(_) => {}
                    Err(err) => {
                        return err.into_response();
                    }
                };

                return (StatusCode::NOT_FOUND, "Paste not found").into_response();
            } else {
                return (StatusCode::BAD_GATEWAY, "Google Drive wouldn't talk to us!")
                    .into_response();
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "public, max-age=15552000".parse().unwrap());

        let mut our_response = Response::new(Body::from_stream(response.bytes_stream()));
        *our_response.headers_mut() = headers;
        our_response
    } else {
        (StatusCode::NOT_FOUND, "Paste not found").into_response()
    }
}

async fn search(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    cookies: Cookies,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if !params.contains_key("tags") {
        return (StatusCode::BAD_REQUEST, "No tags parameter found").into_response();
    }

    let tags = paste::fix_tags(&params.get("tags").map(|s| s.to_owned()));
    if tags.is_empty() {
        return (StatusCode::BAD_REQUEST, "Tags parameter is empty").into_response();
    }

    let page: i64 = params
        .get("page")
        .map(|s| s.parse().unwrap_or(1))
        .unwrap_or(1);

    if !headers.contains_key("X-Requested-With") {
        let (user_id, _) = utils::get_user_id(&state, &cookies);
        let template = templates::SearchTemplate {
            static_domain: state.config.static_domain.clone(),
            user_id,
        };
        return (StatusCode::OK, template).into_response();
    }

    let pastes = match paste::Paste::search(&state.db, &tags, page).await {
        Ok(pastes) => pastes,
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, err).into_response();
        }
    };

    #[derive(Serialize)]
    #[serde(untagged)]
    enum ResponseValue {
        Pastes(Vec<paste::SearchPaste>),
        Tags(Vec<String>),
        Page(i64),
    }

    let mut response: HashMap<&str, ResponseValue> = HashMap::new();
    response.insert("page", ResponseValue::Page(page + 1));
    response.insert("pastes", ResponseValue::Pastes(pastes));
    response.insert("tags", ResponseValue::Tags(tags));

    (StatusCode::OK, Json(response)).into_response()
}

async fn logout(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    let cookies = cookies.private(&state.cookie_key);
    cookies.remove(utils::build_auth_cookie(&state, "".to_string()));
    (StatusCode::SEE_OTHER, [(LOCATION, "/pastebin/")], "").into_response()
}

async fn robots() -> impl IntoResponse {
    (
        StatusCode::OK,
        "User-agent: *\nDisallow: /*/content\nAllow: /",
    )
        .into_response()
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    (StatusCode::NOT_FOUND, template)
}
