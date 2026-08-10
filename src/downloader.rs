use std::path::{Path, PathBuf};
use std::process::Stdio;

use reqwest::Client;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::task::JoinSet;

use crate::bilibili::{DEFAULT_REFERER, DEFAULT_USER_AGENT, VIDEO_URL};
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct Downloader {
    http: Client,
    cookie: String,
}

pub struct MergedVideo {
    pub path: PathBuf,
    #[allow(dead_code)]
    temp_dir: tempfile::TempDir, // kept alive for cleanup-on-drop
}

impl MergedVideo {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Downloader {
    pub fn new(http: Client, cookie: impl Into<String>) -> Self {
        Self {
            http,
            cookie: cookie.into(),
        }
    }

    pub async fn download_and_merge(
        &self,
        video_url: &str,
        audio_url: &str,
        bvid: &str,
    ) -> AppResult<MergedVideo> {
        let temp_dir = tempfile::Builder::new()
            .prefix("bilibili_downloader_")
            .tempdir()
            .map_err(|e| {
                AppError::Internal(format!("Failed to create temporary directory: {e}"))
            })?;

        let video_path = temp_dir.path().join("video.mp4");
        let audio_path = temp_dir.path().join("audio.m4a");
        let output_path = temp_dir.path().join("output.mp4");
        let referer = format!("{VIDEO_URL}/video/{bvid}/");

        let mut set = JoinSet::new();
        {
            let this_video = self.clone_client_state();
            let video_url = video_url.to_string();
            let referer = referer.clone();
            let video_path = video_path.clone();
            set.spawn(async move {
                this_video
                    .download_file(&video_url, &referer, &video_path)
                    .await
                    .map_err(|e| AppError::Download(format!("Video download failed: {e}")))
            });
        }
        {
            let this_audio = self.clone_client_state();
            let audio_url = audio_url.to_string();
            let referer = referer.clone();
            let audio_path = audio_path.clone();
            set.spawn(async move {
                this_audio
                    .download_file(&audio_url, &referer, &audio_path)
                    .await
                    .map_err(|e| AppError::Download(format!("Audio download failed: {e}")))
            });
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(AppError::Internal(format!("Download task join error: {e}")));
                }
            }
        }

        self.merge_with_ffmpeg(&video_path, &audio_path, &output_path)
            .await?;

        // Remove intermediate tracks; keep output.
        let _ = fs::remove_file(&video_path).await;
        let _ = fs::remove_file(&audio_path).await;

        Ok(MergedVideo {
            path: output_path,
            temp_dir,
        })
    }

    fn clone_client_state(&self) -> Self {
        Self {
            http: self.http.clone(),
            cookie: self.cookie.clone(),
        }
    }

    async fn download_file(&self, url: &str, referer: &str, filename: &Path) -> Result<(), String> {
        let resp = self
            .http
            .get(url)
            .header("User-Agent", DEFAULT_USER_AGENT)
            .header(
                "Referer",
                if referer.is_empty() {
                    DEFAULT_REFERER
                } else {
                    referer
                },
            )
            .header("Cookie", &self.cookie)
            .header("Accept-Encoding", "identity")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Download failed, status code: {}", resp.status()));
        }

        let mut file = fs::File::create(filename)
            .await
            .map_err(|e| format!("Failed to create file: {e}"))?;

        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read chunk: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write file: {e}"))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {e}"))?;
        Ok(())
    }

    async fn merge_with_ffmpeg(
        &self,
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> AppResult<()> {
        let output = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(video_path)
            .arg("-i")
            .arg(audio_path)
            .arg("-c")
            .arg("copy")
            .arg(output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(format!("FFmpeg not found or failed to start: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Ffmpeg(format!(
                "FFmpeg execution failed: {stderr}"
            )));
        }
        Ok(())
    }
}

pub async fn check_ffmpeg_installed() -> AppResult<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| AppError::Ffmpeg(format!("FFmpeg not installed or unavailable: {e}")))?;

    if !output.success() {
        return Err(AppError::Ffmpeg(
            "FFmpeg not installed or unavailable".into(),
        ));
    }
    Ok(())
}
