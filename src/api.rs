use crate::errors::PastebinError;
use crate::forms;
use crate::paste;
use crate::runtime;
use crate::templates;
use crate::utils;
use axum::extract::{Json as JsonForm, Path, State};
use axum_extra::{TypedHeader, headers::Host};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;
use sqlx::Error::RowNotFound;
use sqlx::{query, query_scalar};
use std::sync::Arc;
use tower_cookies::Cookies;
use tracing::{error, warn};

const DAILY_LIMIT: u8 = 50; // we allow 50 pastes per user per day

async fn get_count(state: &Arc<runtime::AppState>, user_id: &str) -> u8 {
    let count: i64 = match query_scalar!(
        r#"SELECT COUNT(id) FROM pastebin WHERE user_id=$1 AND date::date = CURRENT_DATE"#,
        user_id
    ).fetch_one(&state.db).await {
        Ok(cnt) => cnt.unwrap_or(0),
        Err(err) => {
            error!("Failed to fetch daily count for {}: {:?}", user_id, err);
            0
        },
    };

    count.clamp(0, u8::MAX as i64) as u8
}

/// Postgres is the source of truth for the daily count (there's a user_id/date index
/// for it), so the limit holds no matter how many instances are running.
async fn rate_limited(state: &Arc<runtime::AppState>, user_id: &str) -> bool {
    get_count(state, user_id).await >= DAILY_LIMIT
}

#[derive(Serialize)]
struct APISuccess {
    success: bool,
    paste_id: String,
    url: String,
}

async fn identify_user(
    state: &Arc<runtime::AppState>,
    headers: HeaderMap,
) -> Result<(String, String), PastebinError> {
    let token = headers.get("Authorization");
    if token.is_none() {
        return Err(PastebinError::Validation("Missing API token!".to_string()));
    }

    let token = match token.unwrap().to_str() {
        Ok(t) => match t.split_whitespace().last() {
            Some(token) => token.to_string(),
            None => {
                return Err(PastebinError::Validation(
                    "Invalid token format! Expected: Bearer <token>".to_string(),
                ))
            }
        },
        Err(_) => {
            return Err(PastebinError::Validation(
                "Invalid token encoding!".to_string(),
            ))
        }
    };

    let cookies = Cookies::default();
    cookies.add(utils::build_auth_cookie(state, token.to_string()));
    let (user_id, session_id) = utils::get_user_id(state, &cookies);
    let (user_id, session_id) = match (user_id, session_id) {
        (Some(uid), Some(sid)) => (uid, sid),
        _ => return Err(PastebinError::Auth("Invalid API token!".to_string())),
    };

    // Check if the user is rate limited
    if rate_limited(state, &user_id).await {
        return Err(PastebinError::TooMany(
            "Eep slow down! Come back tomorrow!@".to_string(),
        ));
    }

    // Check if user's token has not been revoked
    let token_present: bool = match query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM api_tokens WHERE user_id=$1 AND token=$2)"#,
        &user_id,
        &token,
    ).fetch_one(&state.db).await {
        Ok(present) => present.unwrap_or(false),
        Err(err) => match err {
            RowNotFound => false,
            _ => {
                return Err(PastebinError::Database(err));
            }
        }
    };

    if !token_present {
        warn!("Token not found for user: {} | {}", user_id, token);
        return Err(PastebinError::Auth("Invalid API token! Please generate a new one".to_string()));
    }

    Ok((user_id, session_id))
}

pub async fn create(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    TypedHeader(hostname): TypedHeader<Host>,
    JsonForm(payload): JsonForm<forms::PasteAPIForm>,
) -> Result<Response, PastebinError> {
    let (user_id, session_id) = identify_user(&state, headers).await?;

    let payload = forms::PasteForm {
        content: payload.content,
        title: payload.title,
        tags: payload.tags,
        format: payload.format,
        destination: forms::ValidDestination::DataStore,
        csrf_token: "".to_string(),
        token: "".to_string(),
        encrypted: false,
    };

    // Create the paste, use the special score 0.5 for API pastes
    let paste_id = paste::new_paste(
        &state,
        &payload,
        0.5,
        Some(user_id.clone()),
        Some(session_id),
        "",
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(APISuccess {
            success: true,
            url: format!("https://{}/pastebin/{}", hostname, &paste_id),
            paste_id,
        }),
    ).into_response())
}

pub async fn delete(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    TypedHeader(hostname): TypedHeader<Host>,
    Path(paste_id): Path<String>,
) -> Result<Response, PastebinError> {
    let (user_id, _) = identify_user(&state, headers).await?;

    let paste = paste::Paste::get(&state.db, &paste_id).await?;

    if Some(&user_id) == paste.user_id.as_ref() {
        paste.delete(&state).await?;
    } else {
        return Err(PastebinError::Auth("You don't own this paste!".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(APISuccess {
            success: true,
            url: format!("https://{}/pastebin/{}", hostname, &paste_id),
            paste_id,
        }),
    ).into_response())
}

pub async fn about(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> Result<Response, PastebinError> {
    let (user_id, _) = utils::get_user_id(&state, &cookies);

    if user_id.is_none() {
        let template = templates::APIAboutTemplate {
            static_domain: state.config.static_domain.clone(),
            user_id,
            api_key: "".to_string(),
        };
        return Ok(templates::HtmlTemplate(template).into_response());
    }

    let user_id = user_id.unwrap();

    let api_key: String = match query_scalar!(
        r#"SELECT token FROM api_tokens WHERE user_id=$1 LIMIT 1"#,
        &user_id
    ).fetch_one(&state.db).await {
        Ok(token) => token,
        Err(err) => match err {
            RowNotFound => {
                let token = cookies.get(utils::get_cookie_name(&state, "_app_session")
                    .as_str()).map(|c| c.value().to_string()).unwrap_or_default();

                query!(
                    r#"
                    INSERT INTO api_tokens (user_id, token)
                    VALUES ($1, $2)
                    ON CONFLICT (user_id) DO UPDATE
                    SET token=$2
                    "#,
                    &user_id,
                    &token,
                ).execute(&state.db).await?;
                token
            }
            _ => {
                return Err(PastebinError::Database(err));
            }
        },
    };

    let template = templates::APIAboutTemplate {
        static_domain: state.config.static_domain.clone(),
        user_id: Some(user_id),
        api_key,
    };
    Ok(templates::HtmlTemplate(template).into_response())
}
