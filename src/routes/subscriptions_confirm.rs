use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, instrument};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[instrument(name = "Confirm a pending subscription", skip(params))]
pub async fn confirm(
    Query(params): Query<Parameters>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let id = match get_subscriber_id_from_token(&pool, &params.subscription_token).await {
        Ok(id) => id,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    match id {
        None => StatusCode::NOT_FOUND,
        Some(id) => {
            if let Err(e) = confirm_subscriber(&pool, id).await {
                error!("Failed to confirm subscriber: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            StatusCode::OK
        }
    }
}

#[instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
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
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id
        FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

mod error {}
