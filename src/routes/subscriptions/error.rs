use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use tracing::error;

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        match self {
            SubscribeError::ValidationError(e) => {
                (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response()
            }
            SubscribeError::UnexpectedError(e) => {
                error!("Unexpected error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unexpected error: {}", e),
                )
                    .into_response()
            }
        }
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
