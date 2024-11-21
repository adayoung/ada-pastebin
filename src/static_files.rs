use axum::{
    http::header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, VARY, X_CONTENT_TYPE_OPTIONS},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use tracing::warn;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

pub async fn handler(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    // Attempt to get the embedded file
    if let Some(file) = Asset::get(&path) {
        let mut is_brotli = false;
        if path.ends_with(".br") {
            is_brotli = true;
        }

        // Trim .zstd from path
        let path = path.trim_end_matches(".br").to_string();

        // Determine the content type based on the file extension
        let content_type = from_path(&path).first_or_octet_stream();

        // Construct the response headers
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            content_type.as_ref().parse().unwrap_or_else(|err| {
                warn!("Failed to parse content type: {}", err);
                "application/octet-stream".parse().unwrap()
            }),
        );
        headers.insert(X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
        headers.insert(VARY, "Accept-Encoding".parse().unwrap());
        headers.insert(CACHE_CONTROL, "public, max-age=15552000".parse().unwrap());

        if is_brotli {
            headers.insert(CONTENT_ENCODING, "br".parse().unwrap());
        }

        // Return a response with the content type, content encoding, and the file contents
        return (StatusCode::OK, headers, file.data).into_response();
    }

    // Return a 404 response if the file is not found
    (StatusCode::NOT_FOUND, "").into_response()
}
