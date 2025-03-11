use anyhow::Result;
use rusty_ytdl::VideoOptions;
use rusty_ytdl::search::{SearchResult, YouTube};
use rusty_ytdl::{Video, VideoQuality, VideoSearchOptions};
use std::io::{BufReader, Cursor};
use tracing::{debug, info};

pub async fn get_audio_from_youtube(query: &str) -> Result<BufReader<Cursor<Vec<u8>>>> {
    let youtube = YouTube::new().unwrap();

    let res = youtube.search(query, None).await?;

    let Some((id, title)) = res
        .into_iter()
        .filter_map(|x| match x {
            SearchResult::Video(video) => Some((video.id, video.title)),
            _ => None,
        })
        .next()
    else {
        return Err(anyhow::anyhow!("No video found"));
    };

    info!("Found video '{}' with ID {}", title, id);

    download_audio_from_youtube(&id).await
}

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
