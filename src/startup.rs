use axum::{
    Router,
    http::{HeaderName, Request},
    routing::{get, post},
};
use std::time::Duration;
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{debug, error, info, info_span, instrument};

use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{health_check, song, subscribe},
    state::AppState,
};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub struct Application {
    listener: TcpListener,
    pub app: Router,
}

impl Application {
    #[instrument(name = "Building Application", skip_all)]
    pub async fn build(
        Settings {
            database_cfg,
            application_cfg,
            email_client_cfg,
        }: Settings,
    ) -> Result<Self, std::io::Error> {
        info!("Building application.");
        debug!("Database configuration: {:?}", database_cfg);
        let connection_pool = database_cfg.get_pg_pool();
        debug!("Email client configuration: {:?}", email_client_cfg);
        let email_client = EmailClient::try_from(email_client_cfg).expect("Invalid config");

        let listener = application_cfg.listener().await?;
        debug!(
            "Listener bound to port: {}",
            listener.local_addr().unwrap().port()
        );

        let app = Self::get_router(AppState {
            db: connection_pool,
            email_client,
        })
        .await;

        Ok(Self { listener, app })
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    pub async fn get_router(app_state: AppState) -> axum::Router {
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

        Router::new()
            .route("/health_check", get(health_check))
            .route("/songs", post(song))
            .route("/subscriptions", post(subscribe))
            .with_state(app_state)
            .layer(middleware)
            .layer(TimeoutLayer::new(Duration::from_secs(5)))
    }
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        let Application { listener, app } = self;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
