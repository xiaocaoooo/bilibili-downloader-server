mod bilibili;
mod bvid;
mod config;
mod downloader;
mod error;
mod openapi;
mod routes;
mod state;
mod wbi;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::downloader::check_ffmpeg_installed;
use crate::openapi::ApiDoc;
use crate::routes::{docs_redirect, download_video, health};
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = check_ffmpeg_installed().await {
        eprintln!("Error: {e}\nPlease ensure FFmpeg is installed");
        std::process::exit(1);
    }

    tracing::info!("FFmpeg installed");
    tracing::info!("Cookie configured");

    let state = match AppState::new(config.cookie.clone()) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/videos/{id}/download", get(download_video))
        .route("/api/v1/docs", get(docs_redirect))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server starting, listening on: {addr}");
    tracing::info!(
        "Download endpoint: GET http://localhost:{}/api/v1/videos/{{id}}/download",
        config.port
    );
    tracing::info!(
        "Health endpoint:   GET http://localhost:{}/api/v1/health",
        config.port
    );
    tracing::info!(
        "Swagger UI:        http://localhost:{}/swagger-ui/",
        config.port
    );

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind {addr}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Failed to start server: {e}");
        std::process::exit(1);
    }
}
