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
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tower_cookies::Cookies;
use tracing::info;

static OAUTH_CLIENT: OnceLock<BasicClient> = OnceLock::new();

pub fn init_drive_client(state: &Arc<runtime::AppState>) {
    oauth::init_oauth_client(&state.config.drive_oauth, &OAUTH_CLIENT);
}

fn get_oauth_client() -> &'static BasicClient {
    OAUTH_CLIENT.get().expect("Discord client not initialized")
}

static DRIVE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn get_drive_client() -> &'static reqwest::Client {
    DRIVE_CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn auth_start(
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

pub async fn auth_finish(
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
    cookies.add(utils::build_app_cookie(
        &state,
        "_drive_token".to_string(),
        token.access_token().secret().clone(),
        utils::get_cookie_samesite(&state),
    ));

    let template = templates::GDriveTemplate {
        result: "success".to_string(),
    };
    (StatusCode::OK, template).into_response()
}

pub fn get_drive_token(state: &Arc<runtime::AppState>, cookies: &Cookies) -> String {
    let cookies = cookies.private(&state.cookie_key);
    let token = cookies.get(utils::get_cookie_name(state, "_drive_token").as_str());
    if token.is_none() {
        return "".to_string();
    }

    // Nom nom nom!
    cookies.remove(utils::build_app_cookie(
        state,
        "_drive_token".to_string(),
        "".to_string(),
        utils::get_cookie_samesite(state),
    ));

    token.unwrap().value().to_string()
}

pub async fn upload(
    token: &str,
    _content: &[u8],
    _content_type: &str,
    _title: &Option<String>,
    _tags: &Option<Vec<String>>,
    _filename: &str,
) -> Result<(String, String), String> {
    // FIXME: Remove this once we support GDrive uploads
    info!("Token received: {}", token);

    let folder_id = get_pastebin_folder(token).await?;
    info!("Folder ID: {}", folder_id);

    Err("Oop, we don't support Google Drive uploads yet!".to_string())
}

async fn get_pastebin_folder(token: &str) -> Result<String, String> {
    let url = "https://www.googleapis.com/drive/v3/files";
    let params = [
        ("q", "properties has { key='name' and value='Pastebin!!' }"),
        ("fields", "files(id,name)"),
        ("pageSize", "1"),
    ];

    let response = get_drive_client()
        .get(url)
        .bearer_auth(token)
        .query(&params)
        .send()
        .await
        .map_err(|err| format!("Failed to get Pastebin folder: {}", err))?;

    if response.status().is_success() {
        let json_response: Value = response
            .json()
            .await
            .map_err(|err| format!("Failed to parse JSON: {}", err))?;

        // Extract the id from the response
        if let Some(files) = json_response.get("files") {
            if let Some(first_file) = files.get(0) {
                if let Some(id) = first_file.get("id") {
                    return Ok(id.to_string());
                }
            }
        }
    }

    make_pastebin_folder(token).await
}

async fn make_pastebin_folder(token: &str) -> Result<String, String> {
    let url = "https://www.googleapis.com/drive/v3/files";
    let body = json!({
        "name": "Pastebin!!",
        "description": "This folder was made by Ada's HTML Pastebin!",
        "properties": {
            "name": "Pastebin!!",
            "created-by": "https://ada-young.com/pastebin/",
        },
        "mimeType": "application/vnd.google-apps.folder"
    });

    let response = get_drive_client()
        .post(url)
        .bearer_auth(token)
        .json(&body) // Send the JSON body
        .send()
        .await
        .map_err(|err| format!("Failed to make Pastebin folder: {}", err))?;

    if response.status().is_success() {
        let json_response: Value = response
            .json()
            .await
            .map_err(|err| format!("Failed to parse JSON: {}", err))?;

        // Extract the id from the response
        if let Some(id) = json_response.get("id") {
            return Ok(id.to_string());
        } else {
            return Err("No ID found in the response.".to_string());
        }
    }

    Err(response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string()))
}
