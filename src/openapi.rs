use utoipa::OpenApi;

use crate::error::ProblemDetails;
use crate::routes::{DownloadQuery, HealthResponse};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Bilibili Downloader Server",
        version = "0.1.0",
        description = "HTTP API for downloading bilibili videos as merged MP4 files.\n\n\
            Quality codes (qn): 16=360P, 32=480P, 64=720P, 80=1080P, 112=1080P+, 116=1080P60, 120=4K.",
        license(name = "MIT")
    ),
    paths(
        crate::routes::health,
        crate::routes::download_video
    ),
    components(
        schemas(HealthResponse, DownloadQuery, ProblemDetails)
    ),
    tags(
        (name = "system", description = "Service health"),
        (name = "videos", description = "Video download")
    )
)]
pub struct ApiDoc;
