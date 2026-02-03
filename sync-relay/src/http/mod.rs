//! HTTP endpoints for sync-relay.
//!
//! Provides health checks, metrics, and discovery endpoints.

pub mod health;
mod metrics;

use crate::server::SyncRelay;
use axum::{routing::get, Extension, Router};
use std::sync::Arc;

pub use health::HealthStatus;

/// Build the HTTP router with all endpoints.
pub fn build_router(relay: Arc<SyncRelay>) -> Router {
    Router::new()
        .route("/health", get(health::health_handler))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/.well-known/iroh", get(discovery_handler))
        .layer(Extension(relay))
}

/// Discovery endpoint for iroh.
async fn discovery_handler() -> &'static str {
    // Returns minimal discovery info
    // Full implementation would include node address
    "0k-sync relay v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::SqliteStorage;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    async fn test_relay() -> Arc<SyncRelay> {
        let storage = SqliteStorage::in_memory().await.unwrap();
        Arc::new(SyncRelay::new(Config::default(), storage))
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let relay = test_relay().await;
        let app = build_router(relay);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_ok() {
        let relay = test_relay().await;
        let app = build_router(relay);

        let response = app
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn discovery_endpoint_returns_ok() {
        let relay = test_relay().await;
        let app = build_router(relay);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/iroh")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
