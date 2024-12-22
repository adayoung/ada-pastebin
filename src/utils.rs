use crate::runtime;
use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::{IntoResponse, Response},
};
use brotli::CompressorWriter;
use std::io::{Error, Write};
use std::sync::Arc;
use tower_cookies::{cookie::SameSite, Cookie, Cookies};
use tracing::error;

pub async fn extra_sugar(request: Request, next: Next) -> Result<impl IntoResponse, Response> {
    let headers = request.headers().clone();
    let mut response = next.run(request).await;

    let mut sugar = vec![
        ("Ada", String::from("*skips about* Hi! <3 ^_^")),
        ("X-Content-Type-Options", String::from("nosniff")),
        (
            "Referrer-Policy",
            String::from("strict-origin-when-cross-origin"),
        ),
        (
            "Strict-Transport-Security",
            String::from("max-age=31536000"),
        ),
    ];

    // Add Permissions-Policy header
    sugar.push(("Permissions-Policy", generate_permissions_policy()));

    if let Some(user_agent) = headers.get("User-Agent") {
        if let Ok(ua) = user_agent.to_str() {
            if ua.contains("msie") {
                sugar.push(("X-UA-Compatible", String::from("IE=edge,chrome=1")));
                sugar.push(("X-XSS-Protection", String::from("1; mode=block")));
            }
        }
    }

    for (key, value) in sugar {
        if let Ok(v) = HeaderValue::from_str(&value) {
            response.headers_mut().insert(key, v);
        }
    }

    Ok(response)
}

const PERMISSIONS_POLICY: [&str; 9] = [
    "accelerometer=()",
    "camera=()",
    "geolocation=()",
    "gyroscope=()",
    "magnetometer=()",
    "microphone=()",
    "payment=()",
    "usb=()",
    "interest-cohort=()",
];

fn generate_permissions_policy() -> String {
    PERMISSIONS_POLICY.join(",")
}

pub async fn csp(
    State(state): State<Arc<runtime::AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, Response> {
    let mut response = next.run(request).await;
    let static_domain = state.config.static_domain.clone();
    let s3_bucket_url = state.config.s3_bucket_url.clone();

    // FIXME: This is kind of messy, but it works for now
    let policy = vec![
        format!("default-src 'none'"),
        format!("connect-src 'self' {}", s3_bucket_url),
        format!("form-action 'self'"),
        format!("frame-ancestors 'none'"),
        format!(
            "frame-src blob: {} https://challenges.cloudflare.com",
            s3_bucket_url
        ),
        format!("img-src data: {}", static_domain),
        format!(
            "script-src {} https://challenges.cloudflare.com https://cdnjs.cloudflare.com/ajax/libs/xterm/5.5.0/xterm.js",
            static_domain
        ),
        format!("style-src 'unsafe-inline' {} https://cdnjs.cloudflare.com/ajax/libs/xterm/5.5.0/xterm.css", static_domain),
        format!("upgrade-insecure-requests"),
    ];

    if let Ok(csp_header) = HeaderValue::from_str(&policy.join("; ").to_string()) {
        response
            .headers_mut()
            .insert("Content-Security-Policy", csp_header);
    }

    Ok(response)
}

// Compress content using brotli, returning the compressed content and the content encoding
pub async fn compress(content: &str) -> Result<(Vec<u8>, String), Error> {
    if content.len() < 1024 {
        return Ok((content.as_bytes().to_vec(), "identity".to_string()));
    }

    let mut encoder = CompressorWriter::new(Vec::new(), 4096, 6, 22);
    match encoder.write_all(content.as_bytes()) {
        Ok(_) => {}
        Err(err) => {
            return {
                error!("Failed to write compressed content: {}", err);
                Err(err)
            };
        }
    };

    match encoder.flush() {
        Ok(_) => {}
        Err(err) => {
            return {
                error!("Failed to flush compress content: {}", err);
                Err(err)
            };
        }
    };

    Ok((encoder.into_inner(), "br".to_string()))
}

pub fn get_cookie_name(state: &Arc<runtime::AppState>, name: &str) -> String {
    if state.config.cookie_secure {
        format!("__Secure-{}", name)
    } else {
        name.to_string()
    }
}

pub fn get_cookie_samesite(state: &Arc<runtime::AppState>) -> SameSite {
    if state.config.cookie_secure {
        SameSite::Strict
    } else {
        SameSite::Lax
    }
}

pub fn build_auth_cookie<'a>(state: &Arc<runtime::AppState>, value: String) -> Cookie<'a> {
    Cookie::build((get_cookie_name(state, "_app_session"), value))
        .path("/pastebin/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax) // FIXME: This can't be Strict because of the redirect from discord
        .into()
}

pub fn get_user_id(state: &Arc<runtime::AppState>, cookies: &Cookies) -> (Option<String>, Option<String>) {
    let cookies = cookies.private(&state.cookie_key);
    let session_id = cookies.get(get_cookie_name(state, "_app_session").as_str());
    if let Some(session_id) = session_id {
        let session_id = session_id.value().to_string();
        let parts = session_id.split("-ADA-").collect::<Vec<&str>>();
        let user_id = parts.first().map(|s| s.to_string());
        let timestamp = parts.last().map(|s| s.to_string());
        return (user_id, timestamp);
    }

    (None, None)
}
