use axum::{Form, extract::State, response::IntoResponse};
use chrono::Utc;
use hyper::StatusCode;
use rand::{Rng, distr::Alphanumeric, rng};
use reqwest::Url;
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
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

#[instrument(name = "Adding a new song", skip_all, fields(name = %sub.name, email = %sub.email))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<Url>,
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

    let mut transaction = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to start transaction: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to insert subscriber into database: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    let subscription_token = generate_subscription_token();
    if store_token_in_database(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if transaction.commit().await.is_err() {
        error!("Failed to commit transaction");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[instrument(name = "Adding a new subscriber to database", skip_all)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    NewSubscriber { name, email }: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
        subscriber_id,
        email.as_ref(),
        name.as_ref(),
        Utc::now(),
    );
    transaction.execute(query).await.map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

#[instrument(
    name = "Store subscription token in database",
    skip(subscription_token, transaction)
)]
async fn store_token_in_database(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );
    transaction.execute(query).await.map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[instrument(name = "Confirming a subscription", skip(email_client, base_url))]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &Url,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let mut confirmation_link = base_url.join("/subscriptions/confirm").unwrap();
    confirmation_link
        .query_pairs_mut()
        .append_pair("subscription_token", subscription_token)
        .finish();

    let html_body = format!(
        "Welcome to our newsletter!<br />\nClick <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .map_err(|e| {
            error!("Failed to send email: {:?}", e);
            e
        })?;
    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
