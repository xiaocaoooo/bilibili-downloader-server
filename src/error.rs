use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

/// RFC 7807 Problem Details response body.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type.
    #[schema(example = "about:blank")]
    pub r#type: String,
    /// Short, human-readable summary of the problem type.
    #[schema(example = "Not Found")]
    pub title: String,
    /// HTTP status code.
    #[schema(example = 404)]
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    #[schema(example = "Video does not exist")]
    pub detail: String,
    /// URI reference that identifies the specific occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "/api/v1/videos/BVxxxx/download")]
    pub instance: Option<String>,
    /// Application-specific error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "VIDEO_NOT_FOUND")]
    pub code: Option<String>,
}

impl ProblemDetails {
    pub fn new(
        status: StatusCode,
        title: impl Into<String>,
        detail: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            r#type: "about:blank".to_string(),
            title: title.into(),
            status: status.as_u16(),
            detail: detail.into(),
            instance: None,
            code: Some(code.into()),
        }
    }

    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Invalid video ID format")]
    InvalidId,

    #[error("Invalid page parameter: {0}")]
    InvalidPage(String),

    #[error("Invalid quality parameter: {0}")]
    InvalidQuality(String),

    #[error("Video not found: {0}")]
    VideoNotFound(String),

    #[error("Invalid Cookie or insufficient permissions: {0}")]
    Forbidden(String),

    #[error("Upstream API error: {0}")]
    Upstream(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidId | Self::InvalidPage(_) | Self::InvalidQuality(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::VideoNotFound(_) => StatusCode::NOT_FOUND,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Upstream(_) | Self::Download(_) => StatusCode::BAD_GATEWAY,
            Self::Ffmpeg(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::InvalidId | Self::InvalidPage(_) | Self::InvalidQuality(_) => "Bad Request",
            Self::VideoNotFound(_) => "Not Found",
            Self::Forbidden(_) => "Forbidden",
            Self::Upstream(_) | Self::Download(_) => "Bad Gateway",
            Self::Ffmpeg(_) | Self::Internal(_) => "Internal Server Error",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidId => "INVALID_ID",
            Self::InvalidPage(_) => "INVALID_PAGE",
            Self::InvalidQuality(_) => "INVALID_QUALITY",
            Self::VideoNotFound(_) => "VIDEO_NOT_FOUND",
            Self::Forbidden(_) => "INVALID_COOKIE",
            Self::Upstream(_) => "UPSTREAM_ERROR",
            Self::Download(_) => "DOWNLOAD_ERROR",
            Self::Ffmpeg(_) => "FFMPEG_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn into_problem(self, instance: Option<String>) -> ProblemDetails {
        let mut problem = ProblemDetails::new(
            self.status_code(),
            self.title(),
            self.to_string(),
            self.code(),
        );
        if let Some(instance) = instance {
            problem = problem.with_instance(instance);
        }
        problem
    }

    /// Map bilibili / internal error messages to typed errors.
    pub fn from_message(msg: impl AsRef<str>) -> Self {
        let msg = msg.as_ref();
        let lower = msg.to_lowercase();

        if msg.contains("未找到视频")
            || msg.contains("10002")
            || lower.contains("not found")
            || msg.contains("Video information not found")
            || msg.contains("Video page information not found")
            || msg.contains("-404")
        {
            return Self::VideoNotFound(msg.to_string());
        }

        if msg.contains("权限")
            || msg.contains("-403")
            || lower.contains("cookie")
            || lower.contains("permission")
            || lower.contains("login")
        {
            return Self::Forbidden(msg.to_string());
        }

        if lower.contains("ffmpeg") {
            return Self::Ffmpeg(msg.to_string());
        }

        if lower.contains("download") {
            return Self::Download(msg.to_string());
        }

        if lower.contains("api returned") || lower.contains("request failed") {
            return Self::Upstream(msg.to_string());
        }

        Self::Internal(msg.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let problem = self.into_problem(None);
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            Json(problem),
        )
            .into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
