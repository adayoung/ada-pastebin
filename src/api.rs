use crate::forms;
use crate::paste;
use crate::runtime;
use crate::utils;
use axum::extract::{Form, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use serde_json::json;
use std::sync::Arc;
use tower_cookies::Cookies;

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
    Form(payload): Form<forms::PasteAPIForm>,
) -> impl IntoResponse {
    let (user_id, session_id) = match identify_user(&state, headers) {
        Ok(response) => response,
        Err(err) => return err.into_response(),
    };

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
    let paste_id = match paste::new_paste(&state, &payload, 0.9, Some(user_id), Some(session_id)).await {
        Ok(id) => id,
        Err(err) => {
            return err.into_response();
        }
    };

    let response = json!({
        "status": "success",
        "paste_id": paste_id,
    });

    (StatusCode::CREATED, Json(response)).into_response()
}
