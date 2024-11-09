use askama_axum::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

mod static_files;
mod templates;

#[tokio::main]
async fn main() {
    // build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/static/*path", get(static_files::handler))
        .fallback(notfound);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2024").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> templates::BaseTemplate {
    templates::BaseTemplate {}
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = templates::NotFoundTemplate {};
    let rendered = template.render().unwrap(); // Render the 404 template
    (StatusCode::NOT_FOUND, Html(rendered)) // Return a 404 response with the rendered template
}
