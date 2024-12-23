use crate::forms;
use crate::paste;
use crate::runtime;
use crate::utils;
use axum::extract::{Json as JsonForm, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use dashmap::DashSet;
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::time::{sleep, Duration};
use tower_cookies::Cookies;

static RECENT_API_USERS: OnceLock<DashSet<String>> = OnceLock::new();
fn recent_users() -> &'static DashSet<String> {
    RECENT_API_USERS.get_or_init(DashSet::new)
}

#[derive(Serialize)]
struct APISuccess {
    status: String,
    paste_id: String,
}

#[derive(Serialize)]
struct APIError {
    status: String,
    error: String,
}

fn identify_user(
    state: &Arc<runtime::AppState>,
    headers: HeaderMap,
) -> Result<(String, String), (StatusCode, String)> {
    let token = headers.get("Authorization");
    if token.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Missing API token!".to_string()));
    }

    let token = match token.unwrap().to_str() {
        Ok(t) => match t.split_whitespace().last() {
            Some(token) => token.to_string(),
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid token format! Expected: Bearer <token>".to_string(),
                ))
            }
        },
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid token encoding!".to_string(),
            ))
        }
    };

    let cookies = Cookies::default();
    cookies.add(utils::build_auth_cookie(state, token.to_string()));
    let (user_id, session_id) = utils::get_user_id(state, &cookies);
    let (user_id, session_id) = match (user_id, session_id) {
        (Some(uid), Some(sid)) => (uid, sid),
        _ => return Err((StatusCode::UNAUTHORIZED, "Invalid API token!".to_string())),
    };

    Ok((user_id, session_id))
}

pub async fn create(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    JsonForm(payload): JsonForm<forms::PasteAPIForm>,
) -> impl IntoResponse {
    let (user_id, session_id) = match identify_user(&state, headers) {
        Ok(response) => response,
        Err(err) => {
            return (
                err.0,
                Json(APIError {
                    status: "error".to_string(),
                    error: err.1,
                }),
            ).into_response()
        }
    };

    // Check if the user is rate limited
    if recent_users().contains(&user_id) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(APIError {
                status: "error".to_string(),
                error: "Eep slow down!".to_string(),
            }),
        ).into_response();
    }

    let payload = forms::PasteForm {
        content: payload.content,
        title: payload.title,
        tags: payload.tags,
        format: payload.format,
        destination: "datastore".to_string(),
        csrf_token: "".to_string(),
        token: "".to_string(),
    };

    // Create the paste, use the special score 0.9 for API pastes
    let paste_id = match paste::new_paste(
        &state,
        &payload,
        0.9,
        Some(user_id.clone()),
        Some(session_id),
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            return (
                err.0,
                Json(APIError {
                    status: "error".to_string(),
                    error: err.1,
                }),
            ).into_response()
        }
    };

    // Add the user to the recent users list
    recent_users().insert(user_id);

    (
        StatusCode::CREATED,
        Json(APISuccess {
            status: "success".to_string(),
            paste_id,
        }),
    ).into_response()
}

pub async fn delete(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    JsonForm(payload): JsonForm<forms::PasteAPIDeleteForm>,
) -> impl IntoResponse {
    let (user_id, _) = match identify_user(&state, headers) {
        Ok(response) => response,
        Err(err) => return (
                err.0,
                Json(APIError {
                    status: "error".to_string(),
                    error: err.1,
                }),
            ).into_response(),
    };

    // Check if the user is rate limited
    if recent_users().contains(&user_id) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(APIError {
                status: "error".to_string(),
                error: "Eep slow down!".to_string(),
            }),
        ).into_response();
    }

    let paste_id = payload.paste_id;
    let paste = match paste::Paste::get(&state.db, &paste_id).await {
        Ok(paste) => paste,
        Err(err) => {
            return (
                err.0,
                Json(APIError {
                    status: "error".to_string(),
                    error: err.1,
                }),
            ).into_response();
        }
    };

    if Some(&user_id) == paste.user_id.as_ref() {
        match paste.delete(&state).await {
            Ok(_) => {}
            Err(err) => {
                return (
                    err.0,
                    Json(APIError {
                        status: "error".to_string(),
                        error: err.1,
                    }),
                ).into_response();
            }
        };
    } else {
        return (StatusCode::FORBIDDEN, Json(APIError{
            status: "error".to_string(),
            error: "You don't own this paste!".to_string(),
        })).into_response();
    }

    // Add the user to the recent users list
    recent_users().insert(user_id);

    (
        StatusCode::OK,
        Json(APISuccess {
            status: "success".to_string(),
            paste_id,
        }),
    ).into_response()
}

pub async fn reset_api_limiter() {
    loop {
        sleep(Duration::from_secs(30)).await;
        recent_users().clear();
    }
}
