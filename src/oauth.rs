use crate::runtime;
use crate::utils;
use axum::response::{IntoResponse, Redirect};
use oauth2::basic::BasicClient;
use oauth2::{CsrfToken, PkceCodeChallenge, Scope};
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
