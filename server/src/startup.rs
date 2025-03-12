use axum::Router;
use axum::routing::{get, post};
use tokio::net::TcpListener;

use crate::routes::{health_check, song};

pub async fn run(listener: TcpListener) -> Result<(), std::io::Error> {
    // build our application with a single route
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/song", post(song));

    axum::serve(listener, app).await
}
