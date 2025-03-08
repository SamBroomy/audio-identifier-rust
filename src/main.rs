use crate::audio::BandpassFilterMonoSource;
use anyhow::Result;
use audio::{constellation_points, generate_fingerprints, match_fingerprints};
use itertools::Itertools;
use rodio::{Decoder, Source, source};
use std::io::BufReader;
use std::{fs::File, time::Duration};
use tracing::info;

mod audio;
mod model;

use model::{
    find_similar_fingerprints, get_song_info, setup_database, song_exists, store_song_fingerprints,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let pool = setup_database().await?;

    info!("Loading audio...");

    let file = BufReader::new(File::open("Sorry - Waxwing (Official Video).mp3").unwrap());

    let source = BandpassFilterMonoSource::downsample(file)?;

    if song_exists(&pool, "Sorry", "Waxwing").await?.is_none() {
        let constellation_points = constellation_points(source);

        let fingerprints = generate_fingerprints(constellation_points);

        let song_id =
            store_song_fingerprints(&pool, "Sorry", "Waxwing", 0.0, &fingerprints).await?;

        info!("Stored song with ID {}", song_id);
    } else {
        info!("Song already exists in the database");
    }

    let file = BufReader::new(File::open("Sorry - Waxwing (Official Video).mp3").unwrap());
    let source = Decoder::new(file).unwrap();

    let source = Box::new(
        source
            .skip_duration(Duration::from_secs(35))
            .take_duration(Duration::from_secs(25)),
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
