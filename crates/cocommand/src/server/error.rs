use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::CoreError;

/// Standardised API error response body.
///
/// Every error returned by the HTTP layer serialises as:
/// ```json
/// { "ok": false, "error": { "code": "<code>", "message": "<message>" } }
/// ```
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ApiErrorResponse,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiErrorResponse {
    pub ok: bool,
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiErrorResponse {
                ok: false,
                error: ApiErrorBody {
                    code: code.into(),
                    message: message.into(),
                },
            },
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::InvalidInput(msg) => Self::bad_request(msg),
            CoreError::NotImplemented => {
                Self::new(StatusCode::NOT_IMPLEMENTED, "not_implemented", "not implemented")
            }
            CoreError::InvariantViolation(msg) => Self::internal(msg),
            CoreError::Internal(msg) => Self::internal(msg),
        }
    }
}

/// Allow `(StatusCode, String)` error tuples to convert to `ApiError` for
/// gradual migration — existing helpers that produce such tuples keep working.
impl From<(StatusCode, String)> for ApiError {
    fn from((status, message): (StatusCode, String)) -> Self {
        let code = match status {
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::BAD_REQUEST => "bad_request",
            StatusCode::CONFLICT => "conflict",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::NOT_IMPLEMENTED => "not_implemented",
            _ => "internal",
        };
        Self::new(status, code, message)
    }
}
