use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    NotFound(String),

    ///missing header vs unknown key vs revoked key
    #[error("invalid or missing API key")]
    Unauthorized,

    #[error("{0}")]
    Conflict(String),

    #[error("Idempotency-Key was already used with a different request body")]
    IdempotencyKeyReuse,

    #[error("payment processor unavailable: {0}")]
    UpstreamUnavailable(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    message: String,
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "invalid_request",
            AppError::NotFound(_) => "not_found",
            AppError::Unauthorized => "unauthorized",
            AppError::Conflict(_) => "conflict",
            AppError::IdempotencyKeyReuse => "idempotency_key_reuse",
            AppError::UpstreamUnavailable(_) => "upstream_unavailable",
            AppError::Database(_) => "internal_error",
            AppError::Internal(_) => "internal_error",
        }
    }

    fn public_message(&self) -> String {
        match self {
            AppError::Database(_) | AppError::Internal(_) => "an internal error occurred".to_string(),
            other => other.to_string(),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Conflict(_) | AppError::IdempotencyKeyReuse => StatusCode::CONFLICT,
            AppError::UpstreamUnavailable(_) => StatusCode::BAD_GATEWAY,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if matches!(self, AppError::Database(_) | AppError::Internal(_)) {
            tracing::error!(error = %self, "internal error");
        } else {
            tracing::warn!(error = %self, "request rejected");
        }

        HttpResponse::build(self.status_code()).json(ErrorBody {
            error: ErrorDetail {
                kind: self.kind(),
                message: self.public_message(),
            },
        })
    }
}
