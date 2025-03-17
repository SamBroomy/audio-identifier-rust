use server::{configuration::Settings, startup::Application, telemetry::init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber();

    let configuration = Settings::new().expect("Failed to read configuration.");

    let app = Application::build(configuration)
        .await
        .expect("Failed to build application.");

    app.run_until_stopped().await
}
