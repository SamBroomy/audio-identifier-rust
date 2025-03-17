use server::{
    configuration::Settings,
    startup::{get_connection_pool, run},
    telemetry::init_subscriber,
};
use tracing::info;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber();

    let configuration = Settings::new().expect("Failed to read configuration.");
    info!("Loaded configuration: {:?}", configuration);
    let connection_pool = get_connection_pool(&configuration.database);

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    run(listener, connection_pool).await
}
