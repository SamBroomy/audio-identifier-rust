use axum::Form;
use axum::response::IntoResponse;
use hyper::StatusCode;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Song {
    title: String,
    artist: String,
}

pub async fn song(Form(_song): Form<Song>) -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn song_works() {
        let test_song = Song {
            title: "title".to_string(),
            artist: "artist".to_string(),
        };
        let response = song(Form(test_song)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
