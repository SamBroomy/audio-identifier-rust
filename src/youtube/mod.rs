use anyhow::Result;
use rusty_ytdl::VideoOptions;
use rusty_ytdl::{Video, VideoQuality, VideoSearchOptions};

use std::io::{BufReader, Cursor};

mod download;
mod search;

use download::download_audio_from_youtube;
use search::YoutubeSearch;



pub async fn get_audio_from_youtube(query: &str) -> Result<BufReader<Cursor<Vec<u8>>>> {
    let search = YoutubeSearch::new();
    let result = search.search(query).await?;

    download_audio_from_youtube(&result.id).await
}
