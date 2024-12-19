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
use tower_cookies::{cookie::SameSite, Cookie, Cookies, PrivateCookies};

fn build_cookie<'a>(
    state: &Arc<runtime::AppState>,
    name: String,
    value: String,
    path: String,
) -> Cookie<'a> {
    Cookie::build((utils::get_cookie_name(state, name.as_str()), value))
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
        format!("{}-pkce", name),
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
        format!("{}-csrf", name),
        csrf_token.secret().clone(),
        cookie_path.to_string(),
    ));

    Redirect::to(auth_url.as_str())
}

pub async fn finish(
    state: &Arc<runtime::AppState>,
    client: &BasicClient,
    cookies: &Cookies,
    name: &str,
    code: &str,
    csrf_state_param: &str,
    cookie_path: &str,
) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, (StatusCode, String)> {
    let csrf_cookie = utils::get_cookie_name(state, format!("{}-csrf", name).as_str());
    let pkce_cookie = utils::get_cookie_name(state, format!("{}-pkce", name).as_str());

    let cookies = cookies.private(&state.cookie_key);
    let pkce_challenge_secret = cookies.get(pkce_cookie.as_str());
    let csrf_token_secret = cookies.get(csrf_cookie.as_str());

    if pkce_challenge_secret.is_none() || csrf_token_secret.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Missing cookies! Where are teh cookies?!".to_string(),
        ));
    }

    let csrf_token_secret = csrf_token_secret.unwrap().value().to_string();
    if csrf_token_secret != csrf_state_param {
        return Err((StatusCode::FORBIDDEN, "CSRF token mismatch!".to_string()));
    }

    // Reconstruct the PKCE verifier!
    let pkce_verifier = PkceCodeVerifier::new(pkce_challenge_secret.unwrap().value().to_string());

    // Nom nom nom!
    clear_cookies(state, name, cookie_path, cookies);

    // Let's exchange code for token!
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

fn clear_cookies(
    state: &Arc<runtime::AppState>,
    name: &str,
    cookie_path: &str,
    cookies: PrivateCookies,
) {
    cookies.remove(build_cookie(
        state,
        format!("{}-csrf", name),
        "".to_string(),
        cookie_path.to_string(),
    ));

    cookies.remove(build_cookie(
        state,
        format!("{}-pkce", name),
        "".to_string(),
        cookie_path.to_string(),
    ));
}
