//! Local HTTP server: REST API, SSE stream and localhost-only guards.

mod assets;
mod routes;
mod security;

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use tokio::sync::broadcast;

use crate::config::Paths;
use crate::events::LiveEvent;
use crate::store::Store;
use crate::taxonomy::Taxonomy;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub paths: Arc<Paths>,
    pub tax: Arc<Taxonomy>,
    pub live: broadcast::Sender<LiveEvent>,
    pub port: u16,
}

pub fn build_router(state: AppState) -> Router {
    let router = routes::router().fallback(assets::fallback);
    security::guard(router, state.clone()).with_state(state)
}

pub async fn serve(listener: tokio::net::TcpListener, state: AppState) -> anyhow::Result<()> {
    serve_with_shutdown(listener, state, std::future::pending()).await
}

pub async fn serve_with_shutdown(
    listener: tokio::net::TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
