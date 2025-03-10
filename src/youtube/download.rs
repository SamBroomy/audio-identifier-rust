use anyhow::Result;
use rusty_ytdl::VideoOptions;
use rusty_ytdl::{Video, VideoQuality, VideoSearchOptions};
use tracing::{debug, info};

use std::io::{BufReader, Cursor};

pub async fn download_audio_from_youtube(id: &str) -> Result<BufReader<Cursor<Vec<u8>>>> {
    let video_options = VideoOptions {
        quality: VideoQuality::Highest,
        filter: VideoSearchOptions::Audio,
        ..Default::default()
    };
    let video = Video::new_with_options(id, video_options)?;

    info!("Downloading audio from YouTube...");

    let stream = video.stream().await.unwrap();

    let mut audio_buffer = Vec::new();



    while let Some(chunk) = stream.chunk().await.unwrap() {
        audio_buffer.extend_from_slice(&chunk);
        debug!("Downloaded {} bytes so far", audio_buffer.len());
    }

    info!("Downloaded audio from YouTube");
    // Now create a BufReader from the in-memory data
    Ok(BufReader::new(Cursor::new(audio_buffer)))
}
