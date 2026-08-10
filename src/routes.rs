use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use utoipa::{IntoParams, ToSchema};

use crate::bvid::{avid_to_bvid, is_numeric_aid, normalize_bvid};
use crate::error::{AppError, AppResult, ProblemDetails};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "ok")]
    pub status: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct DownloadQuery {
    /// Part number (1-based). Default: 1
    #[serde(default = "default_page")]
    #[param(minimum = 1, example = 1)]
    pub p: i32,
    /// Quality code (qn). Default: 80 (1080P)
    #[serde(default = "default_quality")]
    #[param(minimum = 1, example = 80)]
    pub quality: i32,
}

fn default_page() -> i32 {
    1
}

fn default_quality() -> i32 {
    80
}

/// Health check
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Download a bilibili video as MP4
#[utoipa::path(
    get,
    path = "/api/v1/videos/{id}/download",
    tag = "videos",
    params(
        ("id" = String, Path, description = "BV id (e.g. BV1xx411c7mD) or AV number (e.g. 170001)", example = "BV1xx411c7mD"),
        DownloadQuery
    ),
    responses(
        (status = 200, description = "MP4 video stream", content_type = "video/mp4"),
        (status = 400, description = "Invalid request", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "Invalid cookie or permissions", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Video not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 500, description = "Server error", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 502, description = "Upstream error", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn download_video(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Response {
    let instance = format!("/api/v1/videos/{id}/download");

    if query.p < 1 {
        return problem_response(AppError::InvalidPage(query.p.to_string()), &instance);
    }
    if query.quality < 1 {
        return problem_response(
            AppError::InvalidQuality(query.quality.to_string()),
            &instance,
        );
    }

    match download_inner(&state, &id, query.p, query.quality).await {
        Ok(resp) => resp,
        Err(err) => problem_response(err, &instance),
    }
}

fn problem_response(err: AppError, instance: &str) -> Response {
    let status = err.status_code();
    let problem = err.into_problem(Some(instance.to_string()));
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        Json(problem),
    )
        .into_response()
}

fn resolve_bvid(id: &str) -> AppResult<String> {
    if let Some(bvid) = normalize_bvid(id) {
        if bvid.len() < 3 {
            return Err(AppError::InvalidId);
        }
        return Ok(bvid);
    }
    if is_numeric_aid(id) {
        let aid: u64 = id.parse().map_err(|_| AppError::InvalidId)?;
        return Ok(avid_to_bvid(aid));
    }
    Err(AppError::InvalidId)
}

async fn download_inner(
    state: &AppState,
    id: &str,
    page: i32,
    quality: i32,
) -> AppResult<Response> {
    let bvid = resolve_bvid(id)?;
    let cid = state.bilibili.get_cid(&bvid, page).await?;
    let play = state.bilibili.get_play_url(&bvid, cid, quality).await?;

    if play.dash.video.is_empty() || play.dash.audio.is_empty() {
        return Err(AppError::Upstream("No video or audio stream found".into()));
    }

    let video_url = play.dash.video[0]
        .best_url()
        .ok_or_else(|| AppError::Upstream("Video URL is empty".into()))?
        .to_string();
    let audio_url = play.dash.audio[0]
        .best_url()
        .ok_or_else(|| AppError::Upstream("Audio URL is empty".into()))?
        .to_string();

    let merged = state
        .downloader
        .download_and_merge(&video_url, &audio_url, &bvid)
        .await?;

    let file = File::open(merged.path())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to open output file: {e}")))?;
    let reader = ReaderStream::new(file);

    let guarded =
        futures::stream::unfold((reader, Some(merged)), |(mut reader, guard)| async move {
            match reader.next().await {
                Some(item) => Some((item, (reader, guard))),
                None => {
                    drop(guard);
                    None
                }
            }
        });

    let body = Body::from_stream(guarded);
    let filename = format!("{bvid}.mp4");
    let disposition = format!("attachment; filename=\"{filename}\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(body)
        .map_err(|e| AppError::Internal(format!("Failed to build response: {e}")))
}

/// Redirect to Swagger UI.
pub async fn docs_redirect() -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/swagger-ui/")
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::FOUND.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_only_bv() {
        assert_eq!(
            normalize_bvid("BV1xx411c7mD").as_deref(),
            Some("BV1xx411c7mD")
        );
        assert_eq!(
            normalize_bvid("bv1xx411c7mD").as_deref(),
            Some("BV1xx411c7mD")
        );
        assert!(normalize_bvid("170001").is_none());
        assert!(normalize_bvid("hello").is_none());
    }

    #[test]
    fn resolve_av_to_bv_classic() {
        assert_eq!(resolve_bvid("2").unwrap(), "BV1xx411c7mD");
        assert_eq!(resolve_bvid("BV1xx411c7mD").unwrap(), "BV1xx411c7mD");
    }
}
