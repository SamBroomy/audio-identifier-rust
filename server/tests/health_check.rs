use secrecy::ExposeSecret;
use server::{
    configuration::{DatabaseSettings, get_configuration},
    startup::create_connection_pool,
    telemetry::init_subscriber,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use tracing::info;
use uuid::Uuid;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    init_subscriber();
});

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection =
        PgConnection::connect(config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let pool = create_connection_pool(config.connection_string().expose_secret())
        .await
        .expect("Failed to create connection pool.");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database.");

    pool
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
async fn spawn_app() -> (String, PgPool) {
    LazyLock::force(&TRACING);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port.");
    info!(
        "Listener bound to random port '{}'",
        listener.local_addr().unwrap()
    );
    let port = listener.local_addr().unwrap().port();
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    info!("Database name: {}", configuration.database.database_name);
    let pool = configure_database(&configuration.database).await;
    let app_pool = pool.clone();

    // Spawn the server in a background task
    tokio::spawn(async move {
        server::startup::run(listener, app_pool)
            .await
            .expect("Failed to run server.");
    });
    (format!("http://127.0.0.1:{}", port), pool)
}

#[tokio::test]
async fn health_check_works() {
    let (address, _) = spawn_app().await;

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
    let (app_address, pool) = spawn_app().await;

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

    let saved = sqlx::query!("SELECT title, artist FROM songs",)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved song.");

    assert_eq!(saved.title, "My Song");
    assert_eq!(saved.artist, "Me");
}

#[tokio::test]
async fn song_returns_a_422_when_data_is_missing() {
    let (app_address, _) = spawn_app().await;
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
