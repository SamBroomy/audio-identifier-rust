use anyhow::{Context, Result};
use chrono::Utc;
use rand::{Rng, distr::Alphanumeric, rng};
use reqwest::Url;
use sqlx::{Executor, Postgres, Transaction};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::{domain::NewSubscriber, email_client::EmailClient};

#[instrument(name = "Adding a new subscriber to database", skip_all)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    NewSubscriber { name, email }: &NewSubscriber,
) -> Result<Uuid> {
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
    transaction
        .execute(query)
        .await
        .context("A database failure was encountered while trying to store a new subscriber")?;
    Ok(subscriber_id)
}

#[instrument(
    name = "Store subscription token in database",
    skip(subscription_token, transaction)
)]
pub async fn store_token_in_database(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<()> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );
    transaction
        .execute(query)
        .await
        .context("A database failure was encountered while trying to store a subscription token")?;
    Ok(())
}

#[instrument(name = "Confirming a subscription", skip(email_client, base_url))]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &Url,
    subscription_token: &str,
) -> Result<(), anyhow::Error> {
    let mut confirmation_link = base_url.join("/subscriptions/confirm")?;
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
    debug!("Confirmation link: {}", confirmation_link);

    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .context("Failed to send email")
}

pub fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
