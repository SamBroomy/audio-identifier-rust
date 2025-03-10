use crate::audio::BandpassFilterMonoSource;
use anyhow::Result;
use audio::{constellation_points, generate_fingerprints, match_fingerprints};
use itertools::Itertools;
use rodio::{Decoder, Source};
use sqlx::SqlitePool;
use std::time::Duration;
use std::{fmt::Display, io::Seek};
use tracing::{info, instrument};

mod audio;
mod model;
mod youtube;

use model::{
    find_similar_fingerprints, get_song_info, setup_database, song_exists, store_song_fingerprints,
};

#[derive(Debug)]
struct SongInfo {
    title: String,
    artist: String,
}

impl SongInfo {
    fn new(title: impl Into<String>, artist: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            artist: artist.into(),
        }
    }
}

impl Display for SongInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.title, self.artist)
    }
}

#[instrument(skip(pool))]
async fn get_song(pool: &SqlitePool, song: &SongInfo) -> Result<i64> {
    if let Some(song_id) = song_exists(pool, song).await? {
        info!("Song already exists in the database with ID {}", song_id);
        return Ok(song_id);
    }

    let mut buffer = youtube::get_audio_from_youtube(&song.to_string()).await?;
    let file = std::fs::File::create(format!("data/{}.mp3", song))?;
    let mut writer = std::io::BufWriter::new(file);
    std::io::copy(&mut buffer, &mut writer)?;
    buffer.seek(std::io::SeekFrom::Start(0))?; // Reset the buffer to the start

    let source = Decoder::new(buffer)?;



    let source = BandpassFilterMonoSource::downsample(buffer)?;
    let duration = source.total_duration().unwrap().as_secs_f32();

    let constellation_points = constellation_points(source);

    let fingerprints = generate_fingerprints(constellation_points);

    Ok(store_song_fingerprints(pool, song, duration, &fingerprints).await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let pool = setup_database().await?;

    let songs = vec![
        SongInfo::new("Back to friends", "sombr"),
        SongInfo::new("Waxwing", "Sorry"),
        SongInfo::new("Lemon to a Knife Fight", "The Wombats"),
        SongInfo::new("The Less I Know The Better", "Tame Impala"),
        SongInfo::new("Dog Dribble", "Getdown Services"),
        SongInfo::new("The fish needs a bike", "Snapped Ankles"),
    ];

    for song in &songs {
        get_song(&pool, song).await?;
    }

    info!("Loading audio...");

    let song = SongInfo::new("Waxwing", "Sorry");

    let buffer = youtube::get_audio_from_youtube(&song.to_string()).await?;
    let source = Box::new(
        Decoder::new(buffer)?
            .take_duration(Duration::from_secs(25))
            .skip_duration(Duration::from_secs(35)),
    );

    let source = BandpassFilterMonoSource::new(source, 11025);

    let constellation_points = constellation_points(source);

    let fingerprints = generate_fingerprints(constellation_points);

    let potential_matches = find_similar_fingerprints(&pool, &fingerprints).await?;

    let results = match_fingerprints(&fingerprints, potential_matches);

    let song_infos = get_song_info(&pool, &results.iter().map(|r| r.song_id).collect_vec()).await?;
    for result in &results {
        let song_info = song_infos.get(&result.song_id).unwrap();
        info!(
            "Matched song {} by {} with confidence {:.2} at time offset {:.2}",
            song_info.0, song_info.1, result.confidence, result.time_offset
        );
    }

    dbg!(results);

    pool.close().await;

    Ok(())
}
