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
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tower_cookies::Cookies;
use tracing::error;

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
        return (StatusCode::FORBIDDEN, templates::HtmlTemplate(template)).into_response();
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
    templates::HtmlTemplate(template).into_response()
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
    content: &[u8],
    content_type: &str,
    title: &Option<String>,
    tags: &Option<Vec<String>>,
    filename: &str,
) -> Result<(String, String), String> {
    let (gdriveid, gdrivedl) = match upload_to_gdrive(
        token, filename, title, tags, content, content_type
    ).await {
        Ok(response) => response,
        Err(err) => {
            error!("Failed to upload to Google Drive: {}", err);
            return Err(err)
        },
    };

    if let Err(err) = update_permissions(token, &gdriveid).await {
        error!("Failed to update permissions: {}", err);
        return Err(err)
    };

    Ok((gdriveid, gdrivedl))
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
                    return Ok(id.to_string().replace("\"", ""));
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
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Failed to make Pastebin folder: {}", err))?;

    if response.status().is_success() {
        let json_response: Value = response
            .json()
            .await
            .map_err(|err| format!("Failed to parse JSON: {}", err))?;

        // Extract the id from the response
        let id = json_response.get("id")
            .ok_or("No ID found in the response.")?
            .as_str()
            .ok_or("ID is not a string")?
            .replace("\"", "");

        return Ok(id.to_string());
    }

    Err(response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string()))
}

async fn upload_to_gdrive(
    token: &str,
    filename: &str,
    title: &Option<String>,
    tags: &Option<Vec<String>>,
    content: &[u8],
    mime_type: &str,
) -> Result<(String, String), String> {
    let folder_id = get_pastebin_folder(token).await?;

    let url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart";

    // Convert tags to a comma-separated string
    let tags = tags
        .as_ref()
        .map(|tags| tags.join(", "))
        .unwrap_or_default();

    // Prepare the metadata
    let metadata = json!({
        "name": &filename,
        "mimeType": mime_type,
        "parents": [folder_id],
        "description": title,
        "properties": {
            "tags": tags,
        },
    });

    // Prepare request body as per https://developers.google.com/drive/api/guides/manage-uploads#http_1
    let mut body = String::new();
    body.push_str(&format!(
        "--ada-young-com\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n",
        metadata
    ));

    body.push_str(&format!(
        "--ada-young-com\r\nContent-Type: {}\r\n\r\n{}\r\n",
        mime_type,
        String::from_utf8_lossy(content)
    ));

    body.push_str("--ada-young-com--\r\n");

    // Set headers
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_str("multipart/related; boundary=ada-young-com").unwrap());
    headers.insert("Content-Length", HeaderValue::from_str(&body.len().to_string()).unwrap());

    let response = get_drive_client()
        .post(url)
        .query(&[
            ("fields", "id,webContentLink"),
        ])
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|err| format!("Failed to upload to Google Drive: {}", err))?;

    if response.status().is_success() {
        let json_response: Value = response
            .json()
            .await
            .map_err(|err| format!("Failed to parse JSON: {}", err))?;

        // Extract both id and webContentLink from the response
        let id = json_response.get("id")
            .ok_or("No ID found in the response.")?
            .as_str()
            .ok_or("ID is not a string")?
            .replace("\"", "");

        let web_content_link = json_response.get("webContentLink")
            .ok_or("No webContentLink found in the response.")?
            .as_str()
            .ok_or("webContentLink is not a string")?
            .replace("\"", "");

        return Ok((id.to_string(), web_content_link.to_string()));
    }

    Err(response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string()))
}

async fn update_permissions(token: &str, file_id: &str) -> Result<(), String> {
    let url = format!("https://www.googleapis.com/drive/v3/files/{}/permissions", file_id);

    let permissions = json!({
        "role": "reader",
        "type": "anyone",
    });

    let response = get_drive_client()
        .post(url)
        .bearer_auth(token)
        .json(&permissions)
        .send()
        .await
        .map_err(|err| format!("Failed to update permissions: {}", err))?;

    if response.status().is_success() {
        return Ok(());
    }

    Err(response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string()))
}
