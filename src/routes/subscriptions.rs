use axum::{Form, extract::State, response::IntoResponse};
use chrono::Utc;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};

#[derive(Deserialize)]
pub struct SubscriberFormData {
    email: String,
    name: String,
}

impl TryFrom<SubscriberFormData> for NewSubscriber {
    type Error = String;

    fn try_from(sub: SubscriberFormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(sub.name)?;
        let email = SubscriberEmail::parse(sub.email)?;

        Ok(Self { name, email })
    }
}

#[instrument(name = "Adding a new song", skip(pool, sub), fields(name = %sub.name, email = %sub.email))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Form(sub): Form<SubscriberFormData>,
) -> impl IntoResponse {
    info!("Adding new subscriber '{}' - '{}'", sub.name, sub.email);
    let new_subscriber = match sub.try_into() {
        Ok(sub) => sub,
        Err(e) => {
            warn!("Failed to parse subscriber data: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    if let Err(e) = insert_subscriber(&pool, &new_subscriber).await {
        error!("Failed to execute query: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(e) = email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            "Welcome to our newsletter!",
            "Welcome to our newsletter!",
        )
        .await
    {
        error!("Failed to send confirmation email: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[instrument(name = "Adding a new subscriber to database", skip(pool, name, email))]
async fn insert_subscriber(
    pool: &PgPool,
    NewSubscriber { name, email }: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
        Uuid::new_v4(),
        email.as_ref(),
        name.as_ref(),
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
