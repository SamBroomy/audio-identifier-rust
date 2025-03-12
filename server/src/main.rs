use server::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    run(listener).await
}
