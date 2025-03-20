use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, instrument};
use uuid::Uuid;

use crate::error::format_error_details;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[instrument(name = "Confirm a pending subscription", skip(params))]
pub async fn confirm(
    Query(params): Query<Parameters>,
    State(pool): State<PgPool>,
) -> Result<StatusCode, ConfirmError> {
    let id = get_subscriber_id_from_token(&pool, &params.subscription_token).await?;

    match id {
        None => Err(ConfirmError::NotFound),
        Some(id) => {
            confirm_subscriber(&pool, id).await?;
            Ok(StatusCode::OK)
        }
    }
}

#[instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>> {
    sqlx::query!(
        r#"
        SELECT subscriber_id
        FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map(|r| r.map(|r| r.subscriber_id))
    .context("Failed to fetch subscriber_id from token")
}

#[instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await
    .context("Failed to update subscription status")?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmError {
    #[error("A subscriber with this token was not found")]
    NotFound,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for ConfirmError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ConfirmError::NotFound => (
                StatusCode::NOT_FOUND,
                "A subscriber with this token was not found",
            )
                .into_response(),
            ConfirmError::UnexpectedError(ref e) => {
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
