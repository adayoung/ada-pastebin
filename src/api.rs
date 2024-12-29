use crate::forms;
use crate::paste;
use crate::runtime;
use crate::templates;
use crate::utils;
use axum::extract::{Host, Json as JsonForm, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use chrono::Utc;
use scc::HashMap;
use serde::Serialize;
use sqlx::Error::RowNotFound;
use sqlx::{query, query_scalar};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::time::{sleep, Duration};
use tower_cookies::Cookies;
use tracing::{error, warn};

const DAILY_LIMIT: u8 = 50; // we allow 50 requests per user per day

struct RateLimit {
    daily_count: u8,
}

static API_LIMITS: OnceLock<HashMap<String, RateLimit>> = OnceLock::new();
fn api_limits() -> &'static HashMap<String, RateLimit> {
    API_LIMITS.get_or_init(HashMap::new)
}

fn rate_limited(user_id: &str) -> bool {
    let limit = api_limits()
        .entry(user_id.to_string())
        .and_modify(|l| {
            l.daily_count += 1;
        })
        .or_insert(RateLimit {
            // FIXME: initialize daily count from database instead with
            // SELECT COUNT(id) FROM pastebin WHERE user_id=user_id AND date::date=CURRENT_DATE;
            daily_count: 1,
        });

    limit.daily_count > DAILY_LIMIT
}

#[derive(Serialize)]
struct APISuccess {
    success: bool,
    paste_id: String,
    url: String,
}

#[derive(Serialize)]
struct APIError {
    success: bool,
    error: String,
}

async fn identify_user(
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

    // Check if the user is rate limited
    if rate_limited(&user_id) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
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
                error!("Failed to check if token exists: {:?}", err);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to check if token exists!".to_string(),
                ));
            }
        }
    };

    // FIXME: return 403 if token is not present instead of warning
    if !token_present {
        warn!("Token not found for user: {} | {}", user_id, token);
        query!(
            r#"
            INSERT INTO api_tokens (user_id, token)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE
            SET token=$2
            "#,
            &user_id,
            &token,
        )
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!("Failed to save API token: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to save API token: {}", err)
            )
        })?;
    }

    Ok((user_id, session_id))
}

pub async fn create(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    Host(hostname): Host,
    JsonForm(payload): JsonForm<forms::PasteAPIForm>,
) -> impl IntoResponse {
    let (user_id, session_id) = match identify_user(&state, headers).await {
        Ok(response) => response,
        Err(err) => {
            return (
                err.0,
                Json(APIError {
                    success: false,
                    error: err.1,
                }),
            ).into_response()
        }
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

    // Create the paste, use the special score 0.5 for API pastes
    let paste_id = match paste::new_paste(
        &state,
        &payload,
        0.5,
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
                    success: false,
                    error: err.1,
                }),
            ).into_response()
        }
    };

    (
        StatusCode::CREATED,
        Json(APISuccess {
            success: true,
            url: format!("https://{}/pastebin/{}", hostname, &paste_id),
            paste_id,
        }),
    ).into_response()
}

pub async fn delete(
    State(state): State<Arc<runtime::AppState>>,
    headers: HeaderMap,
    Host(hostname): Host,
    Path(paste_id): Path<String>,
) -> impl IntoResponse {
    let (user_id, _) = match identify_user(&state, headers).await {
        Ok(response) => response,
        Err(err) => return (
                err.0,
                Json(APIError {
                    success: false,
                    error: err.1,
                }),
            ).into_response(),
    };

    let paste = match paste::Paste::get(&state.db, &paste_id).await {
        Ok(paste) => paste,
        Err(err) => {
            return (
                err.0,
                Json(APIError {
                    success: false,
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
                        success: false,
                        error: err.1,
                    }),
                ).into_response();
            }
        };
    } else {
        return (StatusCode::FORBIDDEN, Json(APIError{
            success: false,
            error: "You don't own this paste!".to_string(),
        })).into_response();
    }

    (
        StatusCode::OK,
        Json(APISuccess {
            success: true,
            url: format!("https://{}/pastebin/{}", hostname, &paste_id),
            paste_id,
        }),
    ).into_response()
}

pub async fn reset_api() {
    loop {
        let now = Utc::now();
        if let Some(next_midnight) = now.date_naive().succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0)) {
            let duration = (next_midnight - now.naive_utc())
                .to_std()
                .unwrap_or(Duration::from_secs(3600));
            sleep(duration).await;
            api_limits().clear();
        }
    }
}

pub async fn about(
    State(state): State<Arc<runtime::AppState>>,
    cookies: Cookies,
) -> impl IntoResponse {
    let (user_id, _) = utils::get_user_id(&state, &cookies);

    if user_id.is_none() {
        return (StatusCode::OK, templates::APIAboutTemplate {
            static_domain: state.config.static_domain.clone(),
            user_id,
            api_key: "".to_string(),
        }).into_response();
    }

    let user_id = user_id.unwrap();

    let api_key: String = match query_scalar(
        r#"SELECT token FROM api_tokens WHERE user_id=$1 LIMIT 1"#,
    ).bind(&user_id).fetch_one(&state.db).await {
        Ok(token) => token,
        Err(err) => match err {
            RowNotFound => {
                let token = cookies.get(utils::get_cookie_name(&state, "_app_session")
                    .as_str()).map(|c| c.value().to_string()).unwrap_or_default();

                match query!(
                    r#"
                    INSERT INTO api_tokens (user_id, token)
                    VALUES ($1, $2)
                    ON CONFLICT (user_id) DO UPDATE
                    SET token=$2
                    "#,
                    &user_id,
                    &token,
                ).execute(&state.db).await {
                    Ok(_) => token,
                    Err(err) => {
                        error!("Failed to save API token: {:?}", err);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to save API token: {}", err)
                        ).into_response();
                    }
                }
            }
            _ => {
                error!("Failed to fetch API token: {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to fetch API token: {}", err)
                ).into_response();
            }
        },
    };

    (StatusCode::OK, templates::APIAboutTemplate {
        static_domain: state.config.static_domain.clone(),
        user_id: Some(user_id),
        api_key,
    }).into_response()
}
