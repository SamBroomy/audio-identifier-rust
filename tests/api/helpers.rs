use reqwest::Url;
use secrecy::SecretString;
use server::{
    configuration::{DatabaseSettings, Settings},
    startup::Application,
    telemetry::init_subscriber,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use testcontainers_modules::{
    postgres::{self, Postgres},
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};
use tracing::instrument;

use uuid::Uuid;
use wiremock::MockServer;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    init_subscriber();
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub client: reqwest::Client,
    _container: ContainerAsync<Postgres>,
}
impl TestApp {
    /// Spin up an instance of our application
    /// and returns its address (i.e. http://localhost:XXXX)
    #[instrument(name = "Spawning Test App")]
    pub async fn spawn_app() -> TestApp {
        LazyLock::force(&TRACING);
        let mut config = Settings::new().expect("Failed to read configuration");
        // Launch a mock server to stand in for Postmark's API
        let email_server = MockServer::start().await;
        config.email_client_cfg.base_url = Url::parse(&email_server.uri()).unwrap();
        let container = setup_database(&mut config).await;

        // Launch the application as a background task
        let application = Application::build(config.clone())
            .await
            .expect("Failed to build application.");

        let application_port = application.port();

        tokio::spawn(application.run_until_stopped());

        TestApp {
            address: format!("http://localhost:{}", application_port),
            port: application_port,
            db_pool: config.database_cfg.get_pg_pool(),
            email_server,
            client: reqwest::Client::new(),
            _container: container,
        }
    }
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.post("subscriptions", body).await
    }

    pub async fn post_songs(&self, body: String) -> reqwest::Response {
        self.post("songs", body).await
    }

    async fn post(&self, path: &str, body: String) -> reqwest::Response {
        self.client
            .post(format!("{}/{}", &self.address, path))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

/// Starts a Postgres container and configures database settings.
async fn setup_database(config: &mut Settings) -> ContainerAsync<Postgres> {
    const DB_PASSWORD: &str = "password";

    let container = postgres::Postgres::default()
        .with_tag("17-alpine")
        .with_env_var("POSTGRES_PASSWORD", DB_PASSWORD)
        .start()
        .await
        .unwrap();

    let host_port = container.get_host_port_ipv4(5432).await.unwrap();

    // Create app configuration
    config.database_cfg.database_name = format!("test_{}", Uuid::new_v4());
    config.database_cfg.host = "127.0.0.1".into();
    config.database_cfg.require_ssl = false;
    config.database_cfg.username = "postgres".into();
    config.database_cfg.password = SecretString::from(DB_PASSWORD);
    config.database_cfg.port = host_port;
    config.application_cfg.port = 0; // Random port

    // Initialize database
    initialize_database(&config.database_cfg).await;

    container
}

/// Creates database and runs migrations.
async fn initialize_database(config: &DatabaseSettings) {
    // Connect to postgres database first
    let mut connection = PgConnection::connect_with(
        &DatabaseSettings {
            database_name: "postgres".into(),
            username: "postgres".into(),
            password: SecretString::from("password"),
            ..(config.clone())
        }
        .connect_options(),
    )
    .await
    .expect("Failed to connect to Postgres");

    // Create test database
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // Connect to the new database and run migrations
    let pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate database");
}
