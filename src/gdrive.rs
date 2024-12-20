use crate::oauth;
use crate::runtime;
use crate::templates;
use crate::utils;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use oauth2::basic::BasicClient;
use oauth2::TokenResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tower_cookies::{Cookie, Cookies};

static OAUTH_CLIENT: OnceLock<BasicClient> = OnceLock::new();

pub fn init_drive_client(state: &Arc<runtime::AppState>) {
    oauth::init_oauth_client(&state.config.drive_oauth, &OAUTH_CLIENT);
}

fn get_oauth_client() -> &'static BasicClient {
    OAUTH_CLIENT.get().expect("Discord client not initialized")
}

// static DRIVE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
// fn get_drive_client() -> &'static reqwest::Client {
//     DRIVE_CLIENT.get_or_init(reqwest::Client::new)
// }

pub async fn start(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    let client = get_oauth_client();
    oauth::init(
        &state,
        client,
        &cookies,
        "gdrive",
        &state.config.drive_oauth.scopes,
        "/pastebin/auth/gdrive/finish",
    )
}

pub async fn finish(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if params.contains_key("error") {
        let error = params.get("error").unwrap().to_string();
        let template = templates::GDriveTemplate {
            result: format!("{} ☹ Try again!", error),
        };
        return (StatusCode::FORBIDDEN, template).into_response();
    }

    if !params.contains_key("code") || !params.contains_key("state") {
        return (StatusCode::BAD_REQUEST, "No code or state parameter found!").into_response();
    }

    let code = params.get("code").unwrap().to_string();
    let state_param = params.get("state").unwrap().to_string();

    let client = get_oauth_client();
    let token = match oauth::finish(
        &state,
        client,
        &cookies,
        "gdrive",
        &code,
        state_param.as_str(),
        "/pastebin/auth/gdrive/finish",
    )
    .await
    {
        Ok(token) => token,
        Err(err) => {
            return err.into_response();
        }
    };

    let cookies = cookies.private(&state.cookie_key);
    cookies.add(
        Cookie::build((
            utils::get_cookie_name(&state, "_drive_token"),
            token.access_token().secret().clone(),
        ))
        .path("/pastebin/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(utils::get_cookie_samesite(&state))
        .into(),
    );

    let template = templates::GDriveTemplate {
        result: "success".to_string(),
    };
    (StatusCode::OK, template).into_response()
}
