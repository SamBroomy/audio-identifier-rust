use anyhow::{Context, Result};
use axum::{Form, extract::State};
use hyper::StatusCode;
use reqwest::Url;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, instrument};

use super::{
    SubscribeError,
    helpers::{
        generate_subscription_token, insert_subscriber, send_confirmation_email,
        store_token_in_database,
    },
};
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

#[instrument(name = "Adding a new subscriber", skip_all, fields(name = %sub.name, email = %sub.email))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<Url>,
    Form(sub): Form<SubscriberFormData>,
) -> Result<StatusCode, SubscribeError> {
    info!("Adding new subscriber '{}' - '{}'", sub.name, sub.email);
    let new_subscriber = sub.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await?;
    let subscription_token = generate_subscription_token();

    store_token_in_database(&mut transaction, subscriber_id, &subscription_token).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url,
        &subscription_token,
    )
    .await?;
    Ok(StatusCode::OK)
}
