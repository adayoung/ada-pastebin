use crate::runtime;
use crate::utils;
use axum::{
    extract::{Query, State},
    http::header::LOCATION,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use chrono::Utc;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tower_cookies::{Cookie, Cookies};

static OAUTH_CLIENT: OnceLock<BasicClient> = OnceLock::new();

pub fn init_discord_client(state: &Arc<runtime::AppState>) {
    let auth_url =
        AuthUrl::new(state.config.discord_oauth.auth_url.clone()).expect("Invalid auth URL");
    let token_url =
        TokenUrl::new(state.config.discord_oauth.token_url.clone()).expect("Invalid token URL");
    let redirect_url = RedirectUrl::new(state.config.discord_oauth.redirect_url.clone())
        .expect("Invalid redirect URL");

    let client = BasicClient::new(
        ClientId::new(state.config.discord_oauth.client_id.clone()),
        Some(ClientSecret::new(
            state.config.discord_oauth.client_secret.clone(),
        )),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url);

    OAUTH_CLIENT.set(client).unwrap();
}

fn get_client() -> &'static BasicClient {
    OAUTH_CLIENT.get().expect("Discord client not initialized")
}

pub async fn start(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    // Get the client!
    let client = get_client();

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Stuff the PKCE verifier into the cookie!
    let cookies = cookies.private(&state.cookie_key);
    cookies.add(
        Cookie::build((
            utils::get_cookie_name(&state, "discord-pkce"),
            pkce_verifier.secret().clone(),
        ))
        .path("/pastebin/auth/discord/finish")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(utils::get_cookie_samesite(&state))
        .into(),
    );

    // Generate the full authorization URL.
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(state.config.discord_oauth.scopes.clone()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Stuff the CSRF token into the cookie!
    cookies.add(
        Cookie::build((
            utils::get_cookie_name(&state, "discord-csrf"),
            csrf_token.secret().clone(),
        ))
        .path("/pastebin/auth/discord/finish")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(utils::get_cookie_samesite(&state))
        .into(),
    );

    Redirect::to(auth_url.as_str())
}

pub async fn finish(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if !params.contains_key("code") || !params.contains_key("state") {
        return (StatusCode::BAD_REQUEST, "No code or state parameter found!").into_response();
    }

    let code = params.get("code").unwrap().to_string();
    let state_param = params.get("state").unwrap().to_string();

    let cookies = cookies.private(&state.cookie_key);
    let pkce_challenge_secret = cookies.get("discord-pkce");
    let csrf_token_secret = cookies.get("discord-csrf");

    if pkce_challenge_secret.is_none() || csrf_token_secret.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "Missing cookies! Where are teh cookies?!",
        )
            .into_response();
    }

    let csrf_token_secret = csrf_token_secret.unwrap().value().to_string();
    if csrf_token_secret != state_param {
        return (StatusCode::FORBIDDEN, "CSRF token mismatch!").into_response();
    }

    // Reconstruct the PKCE verifier!
    let pkce_verifier = PkceCodeVerifier::new(pkce_challenge_secret.unwrap().value().to_string());

    // Clear the cookies!
    cookies.remove(utils::get_cookie_name(&state, "discord-pkce").into());
    cookies.remove(utils::get_cookie_name(&state, "discord-csrf").into());

    //  Get the client! Again!
    let client = get_client();

    // Exchange the code for a token!
    let token = match client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
    {
        Ok(token) => token,
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)).into_response();
        }
    };

    // Identify who is logged in!
    let user_id = match identify(token.access_token().secret()).await {
        Ok(user_id) => user_id,
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)).into_response();
        }
    };

    let now = Utc::now();
    let session_id = format!("{}-{}", user_id, now.timestamp());
    cookies.add(
        Cookie::build((utils::get_cookie_name(&state, "_app_session"), session_id))
            .path("/pastebin/")
            .http_only(true)
            .secure(state.config.cookie_secure)
            .same_site(utils::get_cookie_samesite(&state))
            .into(),
    );

    (StatusCode::SEE_OTHER, [(LOCATION, "/pastebin/")], "").into_response()
}

#[derive(Deserialize)]
struct User {
    id: String,
}

pub async fn identify(token: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let user = client
        .get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .json::<User>()
        .await?;

    Ok(user.id)
}
