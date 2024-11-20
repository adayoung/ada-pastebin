use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize)]
struct RecaptchaResponse {
    success: bool,
    score: f64,
    action: String,
}

pub async fn verify(secret: &str, action: &str, token: &str) -> Result<f64, reqwest::Error> {
    if !(token.len() > 0) {
        return Ok(0.0);
    }

    let params = [("secret", secret), ("response", token)];
    let client = reqwest::Client::new();
    let response = client
        .post("https://www.google.com/recaptcha/api/siteverify")
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
        return Ok(response.score);
    }

    warn!("Recaptcha verification failed for the token: {}", token);
    Ok(0.0)
}
