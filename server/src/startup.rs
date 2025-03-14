use axum::{
    Router,
    routing::{get, post},
};

use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::debug;

use crate::routes::{health_check, song};

pub async fn run(listener: TcpListener, connection: PgPool) -> Result<(), std::io::Error> {
    debug!("listening on {}", listener.local_addr().unwrap());
    // build our application with a single route

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/song", post(song))
        .with_state(connection)
        .layer(TraceLayer::new_for_http());

    axum::serve(listener, app).await
}

pub async fn create_connection_pool(connection_string: &str) -> Result<PgPool, sqlx::Error> {
    PgPool::connect(connection_string).await
}
