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

pub async fn create(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    Form(payload): Form<forms::PasteForm>,
) -> impl IntoResponse {
    let token = headers.get("Authorization");
    if token.is_none() {
        return (StatusCode::UNAUTHORIZED, "Missing API token!").into_response();
    }

    let token = match token.unwrap().to_str() {
        Ok(t) => match t.split_whitespace().last() {
            Some(token) => token.to_string(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid token format! Expected: Bearer <token>",
                ).into_response()
            }
        },
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid token encoding!").into_response(),
    };
    let cookies = Cookies::default();
    cookies.add(utils::build_auth_cookie(&state, token.to_string()));
    let user_id = utils::get_user_id(&state, &cookies);

    if user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Invalid API token!").into_response();
    }

    // Create the paste, use the special score 0.9 for API pastes
    let paste_id = match paste::new_paste(&state, &payload, 0.9, user_id).await {
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
