use axum::extract::FromRef;
use reqwest::Url;
use sqlx::PgPool;

use crate::email_client::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub email_client: EmailClient,
    pub base_url: Url,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for EmailClient {
    fn from_ref(state: &AppState) -> Self {
        state.email_client.clone()
    }
}

impl FromRef<AppState> for Url {
    fn from_ref(state: &AppState) -> Self {
        state.base_url.clone()
    }
}
