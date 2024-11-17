use axum::{
    http::header::CONTENT_ENCODING,
    http::header::CONTENT_TYPE,
    http::{HeaderMap, StatusCode},
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
        let mut is_zstd = false;
        if path.ends_with(".zst") {
            is_zstd = true;
        }

        // Trim .zstd from path
        let path = path.trim_end_matches(".zst").to_string();

        // Determine the content type based on the file extension
        let content_type = from_path(&path).first_or_octet_stream();

        // Construct the response headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, content_type.as_ref().parse().unwrap());

        if is_zstd {
            headers.insert(CONTENT_ENCODING, "zstd".parse().unwrap());
        }

        // Return a response with the content type, content encoding, and the file contents
        return (StatusCode::OK, headers, file.data).into_response();
    }

    // Return a 404 response if the file is not found
    (StatusCode::NOT_FOUND, "").into_response()
}
