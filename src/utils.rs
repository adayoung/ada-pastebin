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

    // Add Content-Security-Policy header
    sugar.push(("Content-Security-Policy", generate_csp()));

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

fn generate_csp() -> String {
    let csp: Vec<&str> = vec![
        // TODO: Add the CSP stuff here
    ];

    csp.join(";")
}
