use crate::config;
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
        if user_agent.to_str().unwrap().contains("msie") {
            sugar.push(("X-UA-Compatible", String::from("IE=edge,chrome=1")));
            sugar.push(("X-XSS-Protection", String::from("1; mode=block")));
        }
    }

    for (key, value) in sugar {
        response
            .headers_mut()
            .insert(key, HeaderValue::from_str(&value).unwrap());
    }

    Ok(response)
}

fn generate_permissions_policy() -> String {
    let permissions = vec![
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
    State(state): State<Arc<config::AppConfig>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, Response> {
    let mut response = next.run(request).await;

    // FIXME: This is kind of messy, but it works for now
    let policy = vec![
        // format!("default-src {}", state.static_domain),
        String::from("form-action 'self'"),
        String::from("frame-ancestors 'none'"),
        String::from("frame-src 'self' blob: https://www.google.com/recaptcha/ https://recaptcha.google.com/recaptcha/"),
        format!("img-src data: {}", state.static_domain),
        format!("script-src {} https://www.google.com/recaptcha/ https://www.gstatic.com/recaptcha/", state.static_domain),
        format!("style-src 'unsafe-inline' {}", state.static_domain),
        String::from("upgrade-insecure-requests"),
    ];

    response.headers_mut().insert(
        "Content-Security-Policy",
        HeaderValue::from_str(&format!("{}", policy.join("; "))).unwrap(),
    );

    Ok(response)
}
