use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub async fn extra_sugar(request: Request, next: Next) -> Result<impl IntoResponse, Response> {
    let headers = request.headers().clone();
    let mut response = next.run(request).await;

    let mut sugar = vec![
        ("Ada", "*skips about* Hi! <3 ^_^"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "strict-origin-when-cross-origin"),
        ("Strict-Transport-Security", "max-age=31536000"),
        ("Permissions-Policy", "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), interest-cohort=()"),
    ];

    if let Some(user_agent) = headers.get("User-Agent") {
        if user_agent.to_str().unwrap().contains("msie") {
            sugar.push(("X-UA-Compatible", "IE=edge,chrome=1"));
            sugar.push(("X-XSS-Protection", "1; mode=block"));
        }
    }

    for (key, value) in sugar {
        response
            .headers_mut()
            .insert(key, HeaderValue::from_static(value));
    }

    Ok(response)
}
