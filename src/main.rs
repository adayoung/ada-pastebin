use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use axum_csrf::{CsrfConfig, CsrfLayer, CsrfToken, SameSite};
use sqlx::postgres::PgPool;
use std::env;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber;

mod config;
mod forms;
mod paste;
mod recaptcha;
mod runtime;
mod s3;
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

    let shared_state = Arc::new(runtime::AppState {
        config: config::AppConfig::new(),
        db,
    });

    let bind_addr = format!(
        "{}:{}",
        shared_state.config.bind_addr, shared_state.config.port
    );

    let mut csrf_config = CsrfConfig::new()
        .with_cookie_name("csrf")
        .with_cookie_path("/pastebin/")
        .with_cookie_same_site(SameSite::Strict)
        .with_secure(shared_state.config.csrf_secure_cookie);

    if shared_state.config.csrf_secure_cookie {
        csrf_config = csrf_config.with_cookie_name("__Secure-csrf");
    };

    // build our application with routes
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/pastebin/") }))
        .route("/pastebin/", get(pastebin).post(newpaste))
        .route("/pastebin/about", get(about))
        .layer(middleware::from_fn(utils::extra_sugar))
        .layer(middleware::from_fn_with_state(
            shared_state.clone(),
            utils::csp,
        ))
        .layer(CsrfLayer::new(csrf_config))
        .layer(TraceLayer::new_for_http())
        .route("/static/*path", get(static_files::handler))
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

async fn about(State(state): State<Arc<runtime::AppState>>) -> templates::AboutTemplate {
    templates::AboutTemplate {
        static_domain: state.config.static_domain.clone(),
        recaptcha_key: state.config.recaptcha_key.clone(),
    }
}

async fn pastebin(
    State(state): State<Arc<runtime::AppState>>,
    token: CsrfToken,
) -> impl IntoResponse {
    let template = templates::PastebinTemplate {
        static_domain: state.config.static_domain.clone(),
        recaptcha_key: state.config.recaptcha_key.clone(),
        csrf_token: token.authenticity_token().unwrap(),
    };

    (token, template)
}

async fn newpaste(
    State(state): State<Arc<runtime::AppState>>,
    token: CsrfToken,
    headers: axum::http::HeaderMap,
    Form(payload): Form<forms::PasteForm>,
) -> impl IntoResponse {
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

    // Create the paste
    let paste_id = match paste::new_paste(&state, &payload, score).await {
        Ok(id) => id,
        Err(err) => {
            return err.into_response();
        }
    };

    // Check for the presence of the X-Requested-With header
    if headers.contains_key("X-Requested-With") {
        (StatusCode::OK, paste_id).into_response()
    } else {
        (
            StatusCode::SEE_OTHER,
            [("Location", format! {"/pastebin/{}", paste_id})],
            "",
        )
            .into_response()
    }
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    (StatusCode::NOT_FOUND, template)
}
