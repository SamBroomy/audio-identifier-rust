use axum::{Form, extract::State, response::IntoResponse};
use chrono::Utc;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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

#[instrument(name = "Adding a new song", skip(pool, sub), fields(title = %sub.name, artist = %sub.email))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(sub): Form<SubscriberFormData>,
) -> impl IntoResponse {
    info!("Adding new subscriber '{}' - '{}'", sub.name, sub.email);
    let new_subscriber = match sub.try_into() {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to parse subscriber data: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    match insert_subscriber(&pool, new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[instrument(name = "Adding a new subscriber to database", skip(pool, name, email))]
async fn insert_subscriber(
    pool: &PgPool,
    NewSubscriber { name, email }: NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
            "#,
        Uuid::new_v4(),
        email.as_ref(),
        name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
