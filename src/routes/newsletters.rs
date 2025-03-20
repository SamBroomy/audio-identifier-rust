use anyhow::{Context, Result, anyhow};
use axum::{Json, extract::State, response::IntoResponse};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, instrument, warn};

use crate::{domain::SubscriberEmail, email_client::EmailClient, error::format_error_details};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    text: String,
    html: String,
}
//#[instrument(name = "Adding a new subscriber", skip_all, fields(name = %sub.name, email = %sub.email))]
#[instrument(name = "Publish a newsletter", skip_all)]
pub async fn publish_newsletter(
    State(email_client): State<EmailClient>,
    State(pool): State<PgPool>,
    Json(body): Json<BodyData>,
) -> Result<StatusCode, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(PublishError::UnexpectedError)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.text,
                        &body.content.html,
                    )
                    .await
                    .with_context(|| format!("Failed to send email to {}", subscriber.email))
                    .map_err(PublishError::UnexpectedError)?;
            }
            Err(e) => {
                warn!(
                    "Failed to parse subscriber email: {}",
                    format_error_details(&e)
                );
            }
        }
    }
    Ok(StatusCode::OK)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[instrument(name = "Get confirmed subscribers", skip_all)]
async fn get_confirmed_subscribers(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber>>> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch confirmed subscribers")?
    .into_iter()
    .map(|row| match SubscriberEmail::parse(row.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow!(e)),
    })
    .collect();
    Ok(rows)
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Unexpected error: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}
impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(ref e) => {
                error!("Unexpected error: {}", format_error_details(e));
                (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected error").into_response()
            }
        }
    }
}
