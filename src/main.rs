use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;

mod static_files;
mod templates;

struct AppConfig {
    static_domain: String,
}

#[tokio::main]
async fn main() {
    // TODO: this should come from a config file
    let shared_state = Arc::new(AppConfig {
        static_domain: "localhost:2024".to_string(),
    });

    // build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/static/*path", get(static_files::handler))
        .fallback(notfound)
        .with_state(shared_state);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2024").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<AppConfig>>) -> templates::BaseTemplate {
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
