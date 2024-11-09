use axum::{
    http::header::CONTENT_TYPE,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

pub async fn handler(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    // Attempt to get the embedded file
    if let Some(file) = Asset::get(&path) {
        // Determine the content type based on the file extension
        let content_type = from_path(&path).first_or_octet_stream();

        // Return a response with the content type and the file contents
        return (
            StatusCode::OK,
            [(CONTENT_TYPE, content_type.as_ref())],
            file.data,
        )
            .into_response();
    }

    // Return a 404 response if the file is not found
    (StatusCode::NOT_FOUND, "").into_response()
}
