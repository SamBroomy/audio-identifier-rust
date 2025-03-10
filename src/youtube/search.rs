use anyhow::Result;
use serde_json::Value;
use std::env;
use tracing::{error, info};
use url::Url;

#[derive(Debug)]
pub struct VideoSearchResult {
    pub id: String,
    title: String,
}

pub struct YoutubeSearch {
    request: reqwest::RequestBuilder,
}

impl YoutubeSearch {
    pub fn new() -> Self {
        let api_key = env::var("YOUTUBE_API_KEY").expect("YOUTUBE_API_KEY must be set");

        let url = Url::parse_with_params(
            "https://www.googleapis.com/youtube/v3/search",
            &[
                ("key", api_key),
                ("part", "snippet".to_string()),
                ("maxResults", "1".to_string()),
            ],
        )
        .unwrap();

        let client = reqwest::Client::new();
        let request = client.get(url);

        Self { request }
    }

    pub async fn search(&self, query: &str) -> Result<VideoSearchResult> {
        let response = self
            .request
            .try_clone()
            .unwrap()
            .query(&[("q", query)])
            .send()
            .await?;
        if !response.status().is_success() {
            error!("API request failed with status: {}", response.status());
            error!("Response body: {}", response.text().await?);
            panic!("API request failed");
        }

        let json: Value = response.json().await?; // Parse the response body as JSON

        let id = json["items"][0]["id"]["videoId"]
            .as_str()
            .ok_or(anyhow::anyhow!("Video ID not found"))?
            .to_string();
        let title = json["items"][0]["snippet"]["title"]
            .as_str()
            .ok_or(anyhow::anyhow!("Title not found"))?
            .to_string();

        info!("Found video '{}' with ID {}", title, id);

        Ok(VideoSearchResult { id, title })
    }
}
