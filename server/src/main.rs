use secrecy::ExposeSecret;
use server::{
    configuration::get_configuration,
    startup::{create_connection_pool, run},
    telemetry::init_subscriber,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber();

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool =
        create_connection_pool(configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to create connection pool.");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    run(listener, connection_pool).await
}
