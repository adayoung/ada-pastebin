use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use axum_csrf::{CsrfConfig, CsrfLayer, CsrfToken, SameSite};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber;

mod config;
mod forms;
mod paste;
mod recaptcha;
mod static_files;
mod templates;
mod utils;

#[tokio::main]
async fn main() {
    // Set up the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init(); // Initialize the subscriber

    let shared_state = Arc::new(config::AppConfig::new());
    let bind_addr = format!("{}:{}", shared_state.bind_addr, shared_state.port);
    let mut csrf_config = CsrfConfig::new()
        .with_cookie_name("csrf")
        .with_cookie_path("/pastebin/")
        .with_cookie_same_site(SameSite::Strict)
        .with_secure(shared_state.csrf_secure_cookie);

    if shared_state.csrf_secure_cookie {
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
async fn index(State(state): State<Arc<config::AppConfig>>) -> templates::BaseTemplate {
    templates::BaseTemplate {
        static_domain: state.static_domain.clone(),
    }
}

async fn about(State(state): State<Arc<config::AppConfig>>) -> templates::AboutTemplate {
    templates::AboutTemplate {
        static_domain: state.static_domain.clone(),
        recaptcha_key: state.recaptcha_key.clone(),
    }
}

async fn pastebin(
    State(state): State<Arc<config::AppConfig>>,
    token: CsrfToken,
) -> impl IntoResponse {
    let template = templates::PastebinTemplate {
        static_domain: state.static_domain.clone(),
        recaptcha_key: state.recaptcha_key.clone(),
        csrf_token: token.authenticity_token().unwrap(),
    };

    (token, template)
}

async fn newpaste(
    State(state): State<Arc<config::AppConfig>>,
    token: CsrfToken,
    Form(payload): Form<forms::PasteForm>,
) -> impl IntoResponse {
    if token.verify(&payload.csrf_token).is_err() {
        return (StatusCode::FORBIDDEN, "CSRF token is not valid!").into_response();
    }

    let score = recaptcha::verify(&state.recaptcha_secret, "paste", &payload.token)
        .await
        .unwrap_or_else(|err| {
            error!("Error verifying recaptcha: {}", err);
            0.0
        });

    let paste_id = match paste::new_paste(payload, score).await {
        Ok(id) => id,
        Err(err) => {
            return err.into_response();
        }
    };

    (StatusCode::OK, paste_id).into_response()
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    (StatusCode::NOT_FOUND, template)
}
