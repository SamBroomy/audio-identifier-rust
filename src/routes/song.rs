use axum::{Form, extract::State, response::IntoResponse};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::{PgPool, types::BigDecimal};
use tracing::{error, info, instrument};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Song {
    title: String,
    artist: String,
}

#[instrument(name = "Adding a new song", skip(pool, song), fields(title = %song.title, artist = %song.artist))]
pub async fn song(State(pool): State<PgPool>, Form(song): Form<Song>) -> impl IntoResponse {
    info!("Adding new song '{}' by '{}'", song.title, song.artist);

    match insert_subscriber(&pool, song).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[instrument(name = "Adding a new song to database", skip(pool, title, artist))]
async fn insert_subscriber(pool: &PgPool, Song { title, artist }: Song) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO songs (id, title, artist, album, duration)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        Uuid::new_v4(),
        title,
        artist,
        None::<String>,
        BigDecimal::from(0),
        //rust_decimal::Decimal::ZERO,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
