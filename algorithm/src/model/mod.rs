use std::collections::HashMap;

use itertools::Itertools;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use tracing::{info, instrument, warn};

use crate::{SongInfo, audio::Fingerprint};

pub async fn setup_database() -> Result<SqlitePool, sqlx::Error> {
    // Connect to SQLite database (creates it if it doesn't exist)
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data/fingerprints.db")
        .await?;
    sqlx::migrate!().run(&pool).await?;
    // Create tables
    Ok(pool)
}

#[instrument(skip(pool))]
pub async fn song_exists(pool: &SqlitePool, song: &SongInfo) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT id FROM songs WHERE title = ? AND artist = ?",
        song.title,
        song.artist
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|row| row.id))
}

#[instrument(skip(pool, fingerprints))]
pub async fn store_song_fingerprints(
    pool: &SqlitePool,
    song: &SongInfo,
    duration: f32,
    fingerprints: &[Fingerprint],
) -> Result<i64, sqlx::Error> {
    // Begin a transaction
    let mut tx = pool.begin().await?;

    // Check if song exists

    if let Some(song_id) = song_exists(pool, song).await? {
        // Song already exists, return the ID
        warn!("Song already exists: {}", song);
        return Ok(song_id);
    }

    // Insert the song
    let song_id = sqlx::query!(
        "INSERT INTO songs ( title, artist, duration) VALUES ( ?, ?, ?)",
        song.title,
        song.artist,
        duration
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    info!("Inserted new song: {} ID: {}", song, song_id);

    // Insert fingerprints in batches
    for chunk in fingerprints.chunks(1000) {
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO fingerprints (song_id, hash, time_offset, confidence, anchor_frequency, target_frequency, delta_time)",
        );

        query_builder.push_values(chunk, |mut b, fingerprint| {
            let (hash, time_offset, confidence, anchor_freq, target_freq, delta_t): (
                i64,
                f64,
                i64,
                i64,
                i64,
                f64,
            ) = fingerprint.into();

            b.push_bind(song_id)
                .push_bind(hash)
                .push_bind(time_offset)
                .push_bind(confidence)
                .push_bind(anchor_freq)
                .push_bind(target_freq)
                .push_bind(delta_t);
        });

        query_builder.build().execute(&mut *tx).await?;
    }

    // Commit transaction
    tx.commit().await?;

    Ok(song_id)
}

#[instrument(skip(pool, fingerprints))]
pub async fn find_similar_fingerprints(
    pool: &SqlitePool,
    fingerprints: &[Fingerprint],
) -> Result<HashMap<i64, Vec<Fingerprint>>, sqlx::Error> {
    let mut result_map: HashMap<i64, Vec<Fingerprint>> = HashMap::new();

    for chunk in fingerprints.chunks(100) {
        let hashes = chunk.iter().map(|f| f.hash).collect_vec();

        let mut builder = sqlx::QueryBuilder::new(
            "SELECT song_id, hash, time_offset, confidence, anchor_frequency, target_frequency, delta_time FROM fingerprints WHERE ",
        );
        builder.push("hash IN (");
        let mut separated = builder.separated(", ");
        for hash in &hashes {
            separated.push_bind(*hash);
        }
        builder.push(") ORDER BY song_id");
        let rows = builder.build().fetch_all(pool).await?;

        // Process results
        for row in rows {
            let song_id: i64 = row.get("song_id");
            let hash: i64 = row.get("hash");
            let time_offset: f64 = row.get("time_offset");
            let confidence: i64 = row.get("confidence");
            let anchor_freq: i64 = row.get("anchor_frequency");
            let target_freq: i64 = row.get("target_frequency");
            let delta_t: f64 = row.get("delta_time");

            result_map.entry(song_id).or_default().push(
                (
                    hash,
                    time_offset,
                    confidence,
                    anchor_freq,
                    target_freq,
                    delta_t,
                )
                    .into(),
            );
        }
    }
    Ok(result_map)
}

pub async fn get_song_info(
    pool: &SqlitePool,
    song_ids: &[i64],
) -> Result<HashMap<i64, (String, String, u32)>, sqlx::Error> {
    let mut result_map = HashMap::new();

    for chunk in song_ids.chunks(100) {
        let mut builder =
            sqlx::QueryBuilder::new("SELECT id, title, artist, duration FROM songs WHERE ");
        builder.push("id IN (");
        let mut separated = builder.separated(", ");
        for id in chunk {
            separated.push_bind(*id);
        }
        builder.push(")");
        let rows = builder.build().fetch_all(pool).await?;

        // Process results
        for row in rows {
            let id: i64 = row.get("id");
            let title: String = row.get("title");
            let artist: String = row.get("artist");
            let duration: u32 = row.get("duration");

            result_map.insert(id, (title, artist, duration));
        }
    }
    Ok(result_map)
}
