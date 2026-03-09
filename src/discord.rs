use crate::oauth;
use crate::runtime;
use crate::utils;
use crate::errors::PastebinError;
use axum::{
    extract::{Query, State},
    http::header::LOCATION,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use oauth2::{
    basic::BasicClient,
    EndpointSet,
    EndpointNotSet,
    TokenResponse
};
use serde::Deserialize;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tower_cookies::Cookies;

static OAUTH_CLIENT: OnceLock<BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>> = OnceLock::new();

pub fn init_discord_client(state: &Arc<runtime::AppState>) {
    oauth::init_oauth_client(&state.config.discord_oauth, &OAUTH_CLIENT);
}

fn get_oauth_client() -> &'static BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet> {
    OAUTH_CLIENT.get().expect("Discord client not initialized")
}

static IDENTITY_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn get_identity_client() -> &'static reqwest::Client {
    IDENTITY_CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn start(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    let client = get_oauth_client();
    oauth::init(
        &state,
        client,
        &cookies,
        "discord",
        &state.config.discord_oauth.scopes,
        "/pastebin/auth/discord/finish",
    )
}

pub async fn finish(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, PastebinError> {
    if !params.contains_key("code") || !params.contains_key("state") {
        return Err(PastebinError::Validation("No code or state parameter found!".to_string()));
    }

    let code = params.get("code").unwrap().to_string();
    let state_param = params.get("state").unwrap().to_string();

    let client = get_oauth_client();
    let token = oauth::finish(
        &state,
        client,
        &cookies,
        "discord",
        &code,
        state_param.as_str(),
        "/pastebin/auth/discord/finish",
    )
    .await?;

    // Identify who is logged in!
    let user_id = identify(token.access_token().secret()).await?;

    // We want fixed length user_id so we'll use checksum instead
    let user_id = format!("sha256-{}", hex::encode(Sha256::digest(user_id)));

    let now = Utc::now();
    let session_id = format!("{}-ADA-{}", user_id, now.timestamp());

    let cookies = cookies.private(&state.cookie_key);
    cookies.add(utils::build_auth_cookie(&state, session_id));

    Ok((StatusCode::SEE_OTHER, [(LOCATION, "/pastebin/")], "").into_response())
}


#[derive(Deserialize)]
struct User {
    id: String,
}

pub async fn identify(token: &str) -> Result<String, PastebinError> {
    let user = get_identity_client()
        .get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| PastebinError::ExternalService(e.to_string()))?
        .json::<User>()
        .await
        .map_err(|e| PastebinError::ExternalService(e.to_string()))?;

    Ok(user.id)
}
