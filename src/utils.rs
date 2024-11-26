use crate::runtime;
use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

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

fn generate_permissions_policy() -> String {
    let permissions: [&str; 9] = [
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

    permissions.join(",")
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
        format!("frame-src blob: {} https://www.google.com/recaptcha/ https://recaptcha.google.com/recaptcha/", s3_bucket_url),
        format!("img-src data: {}", static_domain),
        format!("script-src {} https://www.google.com/recaptcha/ https://www.gstatic.com/recaptcha/", static_domain),
        format!("style-src 'unsafe-inline' {}", static_domain),
        format!("upgrade-insecure-requests"),
    ];

    if let Ok(csp_header) = HeaderValue::from_str(&policy.join("; ").to_string()) {
        response
            .headers_mut()
            .insert("Content-Security-Policy", csp_header);
    }

    Ok(response)
}
