use server::configuration::get_configuration;
use sqlx::{Connection, PgConnection};

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
async fn spawn_app() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port.");
    let port = listener.local_addr().unwrap().port();
    // Spawn the server in a background task
    tokio::spawn(async move {
        server::startup::run(listener)
            .await
            .expect("Failed to run server.");
    });
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn songs_returns_a_200_for_valid_form_data() {
    let app_address = spawn_app().await;
    let configuration = get_configuration().expect("Failed to read configuration.");
    let mut connection = PgConnection::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let client = reqwest::Client::new();

    let body = "title=My%20Song&artist=Me";
    let response = client
        .post(format!("{}/song", app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT title, artist FROM songs")
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved song.");

    assert_eq!(saved.title, "My Song");
    assert_eq!(saved.artist, "Me");
}

#[tokio::test]
async fn song_returns_a_422_when_data_is_missing() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("title=My%20Song", "missing the artist"),
        ("artist=Me", "missing the title"),
        ("", "missing both the title and artist"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/song", app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            response.status().as_u16(),
            422,
            "The API did not fail with 400 Bad Request when payload was {}.",
            error_message
        );
    }
}
