use crate::runtime;
use crate::utils;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicTokenType, AuthorizationCode, CsrfToken, EmptyExtraTokenFields, PkceCodeChallenge,
    PkceCodeVerifier, Scope, StandardTokenResponse,
};
use std::sync::Arc;
use tower_cookies::{cookie::SameSite, Cookie, Cookies};

fn build_cookie<'a>(
    state: &Arc<runtime::AppState>,
    name: &str,
    value: String,
    path: String,
) -> Cookie<'a> {
    Cookie::build((utils::get_cookie_name(state, name), value))
        .path(path)
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax) // This can't be Strict because of the redirect from oauth provider
        .into()
}

pub fn init(
    state: &Arc<runtime::AppState>,
    client: &BasicClient,
    cookies: &Cookies,
    name: &str,
    scopes: &str,
    cookie_path: &str,
) -> impl IntoResponse {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Stuff the PKCE verifier into the cookie!
    let cookies = cookies.private(&state.cookie_key);
    cookies.add(build_cookie(
        state,
        format!("{}-pkce", name).as_str(),
        pkce_verifier.secret().clone(),
        cookie_path.to_string(),
    ));

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(scopes.to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Stuff the CSRF token into the cookie!
    cookies.add(build_cookie(
        state,
        format!("{}-csrf", name).as_str(),
        csrf_token.secret().clone(),
        cookie_path.to_string(),
    ));

    Redirect::to(auth_url.as_str())
}

pub async fn finish(
    client: &BasicClient,
    code: &str,
    pkce_challenge_secret: &str,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, (StatusCode, String)> {
    let pkce_verifier = PkceCodeVerifier::new(pkce_challenge_secret.to_string());

    match client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
    {
        Ok(token) => Ok(token),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))),
    }
}
