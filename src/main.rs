use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;

mod config;
mod static_files;
mod templates;

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(config::AppConfig::new());
    let bind_addr = format!("{}:{}", shared_state.bind_addr, shared_state.port);

    // build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/static/*path", get(static_files::handler))
        .fallback(notfound)
        .with_state(shared_state);

    // run it
    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<config::AppConfig>>) -> templates::BaseTemplate {
    templates::BaseTemplate {
        static_domain: state.static_domain.clone(),
    }
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    let rendered = template.render().unwrap(); // Render the 404 template
    (StatusCode::NOT_FOUND, Html(rendered)) // Return a 404 response with the rendered template
}
