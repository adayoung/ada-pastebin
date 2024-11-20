use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber;

mod config;
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

    // build our application with routes
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/pastebin/") }))
        .route("/pastebin/", get(pastebin))
        .route("/pastebin/about", get(about))
        .layer(middleware::from_fn(utils::extra_sugar))
        .layer(middleware::from_fn_with_state(
            shared_state.clone(),
            utils::csp,
        ))
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

async fn pastebin(State(state): State<Arc<config::AppConfig>>) -> templates::PastebinTemplate {
    templates::PastebinTemplate {
        static_domain: state.static_domain.clone(),
        recaptcha_key: state.recaptcha_key.clone(),
    }
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    let rendered = template.render().unwrap(); // Render the 404 template
    (StatusCode::NOT_FOUND, Html(rendered)) // Return a 404 response with the rendered template
}
