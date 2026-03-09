use crate::utils::not_found_response;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum PastebinError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Payload too large: {0}")]
    TooBig(String),

    #[error("Too many requests: {0}")]
    TooMany(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for PastebinError {
    fn into_response(self) -> Response {
        let status = match &self {
            PastebinError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PastebinError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PastebinError::Auth(_) => StatusCode::UNAUTHORIZED,
            PastebinError::Forbidden(_) => StatusCode::FORBIDDEN,
            PastebinError::Validation(_) => StatusCode::BAD_REQUEST,
            PastebinError::NotFound(_) => StatusCode::NOT_FOUND,
            PastebinError::ExternalService(_) => StatusCode::BAD_GATEWAY,
            PastebinError::TooBig(_) => StatusCode::PAYLOAD_TOO_LARGE,
            PastebinError::TooMany(_) => StatusCode::TOO_MANY_REQUESTS,
            PastebinError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Log internal errors
        match &self {
            PastebinError::Database(e) => {
                error!("Database error: {}", e);
            }
            PastebinError::Storage(e) => {
                error!("Storage error: {}", e);
            }
            PastebinError::Internal(e) => {
                error!("Internal error: {}", e);
            }
            PastebinError::ExternalService(e) => {
                error!("External service error: {}", e);
            }
            _ => {} // Don't log user-facing errors
        }

        // For internal errors, don't expose details to the client
        let message = match &self {
            PastebinError::Database(_) => "Internal server error".to_string(),
            PastebinError::Storage(_) => "Internal server error".to_string(),
            PastebinError::Internal(_) => "Internal server error".to_string(),
            _ => self.to_string(),
        };

        if status == StatusCode::NOT_FOUND {
            return not_found_response();
        }

        (status, message).into_response()
    }
}
