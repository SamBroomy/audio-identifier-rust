use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use tracing::{error, warn};

use crate::error::format_error_details;

#[derive(Debug, thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        match self {
            SubscribeError::ValidationError(e) => {
                warn!("Validation error: {}", e);
                (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response()
            }
            SubscribeError::UnexpectedError(ref e) => {
                error!("Unexpected error: {}", format_error_details(e));
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unexpected error: {}", e),
                )
                    .into_response()
            }
        }
    }
}
