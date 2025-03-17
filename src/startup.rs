use axum::{
    Router,
    http::{HeaderName, Request},
    routing::{get, post},
};

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{debug, error, info_span};

use crate::{
    configuration::DatabaseSettings,
    routes::{health_check, song, subscribe},
};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn run(listener: TcpListener, connection: PgPool) -> Result<(), std::io::Error> {
    debug!("listening on {}", listener.local_addr().unwrap());

    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                // Log the request id as generated.
                let request_id = request.headers().get(REQUEST_ID_HEADER);

                match request_id {
                    Some(request_id) => info_span!(
                        "http_request",
                        request_id = ?request_id,
                    ),
                    None => {
                        error!("could not extract request_id");
                        info_span!("http_request")
                    }
                }
            }),
        )
        // send headers from request to response headers
        .layer(PropagateRequestIdLayer::new(x_request_id));

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/song", post(song))
        .route("/subscriptions", post(subscribe))
        .with_state(connection)
        .layer(middleware);

    axum::serve(listener, app).await
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.connect_options())
}
