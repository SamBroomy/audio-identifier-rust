use axum::response::IntoResponse;
use hyper::StatusCode;
use tracing::instrument;

#[instrument(name = "Health Check", ret)]
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn health_check_works() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
