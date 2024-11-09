use askama_axum::Template;
use axum::{
    http::header::CONTENT_TYPE,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate;

#[tokio::main]
async fn main() {
    // build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/static/*path", get(static_files))
        .fallback(notfound);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> BaseTemplate {
    BaseTemplate {}
}

// Fallback handler for 404 errors
async fn notfound() -> impl IntoResponse {
    let template = NotFoundTemplate {};
    let rendered = template.render().unwrap(); // Render the 404 template
    (StatusCode::NOT_FOUND, Html(rendered)) // Return a 404 response with the rendered template
}

async fn static_files(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    // Attempt to get the embedded file
    if let Some(file) = Asset::get(&path) {
        // Determine the content type based on the file extension
        let content_type = from_path(&path).first_or_octet_stream();

        // Return a response with the content type and the file contents
        return (
            axum::http::StatusCode::OK,
            [(CONTENT_TYPE, content_type.as_ref())],
            file.data,
        )
            .into_response();
    }

    // Return a 404 response if the file is not found
    (axum::http::StatusCode::NOT_FOUND, "").into_response()
}
