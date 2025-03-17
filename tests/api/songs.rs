use crate::helpers::TestApp;

#[tokio::test]
async fn songs_returns_a_200_for_valid_form_data() {
    let app = TestApp::spawn_app().await;
    let client = reqwest::Client::new();

    let body = "title=My%20Song&artist=Me";

    let response = app.post_songs(body.into()).await;

    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT title, artist FROM songs",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved song.");

    assert_eq!(saved.title, "My Song");
    assert_eq!(saved.artist, "Me");
}

#[tokio::test]
async fn song_returns_a_422_when_data_is_missing() {
    let app = TestApp::spawn_app().await;
    let test_cases = vec![
        ("title=My%20Song", "missing the artist"),
        ("artist=Me", "missing the title"),
        ("", "missing both the title and artist"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_songs(invalid_body.into()).await;

        assert_eq!(
            response.status().as_u16(),
            422,
            "The API did not fail with 400 Bad Request when payload was {}.",
            error_message
        );
    }
}
