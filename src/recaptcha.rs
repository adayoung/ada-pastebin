use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize)]
struct RecaptchaResponse {
    success: bool,
    // score: f64,
    action: String,
}

fn is_debug() -> bool {
    #[cfg(debug_assertions)]
    {
        true
    }

    #[cfg(not(debug_assertions))]
    {
        false
    }
}

pub async fn verify(secret: &str, action: &str, token: &str) -> Result<f64, reqwest::Error> {
    if token.is_empty() || is_debug() {
        return Ok(0.0);
    }

    let params = [("secret", secret), ("response", token)];
    let client = reqwest::Client::new();
    let response = client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .form(&params)
        .send()
        .await?
        .json::<RecaptchaResponse>()
        .await?;

    if response.success {
        if response.action != action {
            warn!(
                "Recaptcha action mismatch: {} != {}",
                response.action, action
            );
            return Ok(0.0);
        }
        return Ok(0.7); // FIXME: Turnstile doesn't return a score
    }

    warn!("Recaptcha verification failed for the token: {}", token);
    Ok(0.0)
}
