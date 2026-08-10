use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::error::{AppError, AppResult};
use crate::wbi::{self, WbiKeys};

pub const BASE_URL: &str = "https://api.bilibili.com";
pub const VIDEO_URL: &str = "https://www.bilibili.com";
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const DEFAULT_REFERER: &str = "https://www.bilibili.com/";

const PAGELIST_ENDPOINT: &str = "/x/player/pagelist";
const NAV_ENDPOINT: &str = "/x/web-interface/nav";
const PLAYURL_ENDPOINT: &str = "/x/player/wbi/playurl";

const DEFAULT_FNVER: i32 = 0;
const DEFAULT_FNVAL: i32 = 4048;
const DEFAULT_FOURK: i32 = 1;

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct ApiResponse<T> {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<T>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct CidInfo {
    pub cid: i64,
    pub page: i32,
    #[serde(default)]
    pub part: String,
    #[serde(default)]
    pub duration: i32,
}

#[derive(Debug, Deserialize)]
pub struct NavData {
    pub wbi_img: WbiImgData,
}

#[derive(Debug, Deserialize)]
pub struct WbiImgData {
    pub img_url: String,
    pub sub_url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PlayUrlData {
    pub dash: DashData,
    #[serde(default)]
    pub quality: i32,
    #[serde(default)]
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DashData {
    #[serde(default)]
    pub video: Vec<MediaTrack>,
    #[serde(default)]
    pub audio: Vec<MediaTrack>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct MediaTrack {
    #[serde(default)]
    pub id: i32,
    #[serde(default, alias = "base_url", alias = "baseUrl")]
    pub base_url: String,
    #[serde(default, alias = "backup_url", alias = "backupUrl")]
    pub backup_url: Vec<String>,
}

impl MediaTrack {
    pub fn best_url(&self) -> Option<&str> {
        if !self.base_url.is_empty() {
            Some(self.base_url.as_str())
        } else {
            self.backup_url.first().map(String::as_str)
        }
    }
}

#[derive(Clone)]
pub struct BilibiliClient {
    cookie: String,
    http: Client,
    wbi_keys: Arc<RwLock<Option<WbiKeys>>>,
}

impl BilibiliClient {
    pub fn new(cookie: impl Into<String>) -> AppResult<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            cookie: cookie.into(),
            http,
            wbi_keys: Arc::new(RwLock::new(None)),
        })
    }

    pub fn http(&self) -> &Client {
        &self.http
    }

    fn apply_headers(
        &self,
        req: reqwest::RequestBuilder,
        referer: Option<&str>,
    ) -> reqwest::RequestBuilder {
        req.header("User-Agent", DEFAULT_USER_AGENT)
            .header("Referer", referer.unwrap_or(DEFAULT_REFERER))
            .header("Cookie", &self.cookie)
    }

    pub async fn get_cid(&self, bvid: &str, page: i32) -> AppResult<i64> {
        let url = format!("{BASE_URL}{PAGELIST_ENDPOINT}?bvid={bvid}");
        let req = self.apply_headers(self.http.get(&url), None);
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("Request failed: {e}")))?;
        let body = resp
            .error_for_status()
            .map_err(|e| AppError::Upstream(format!("HTTP error: {e}")))?
            .text()
            .await
            .map_err(|e| AppError::Upstream(format!("Failed to read response body: {e}")))?;

        let parsed: ApiResponse<Vec<CidInfo>> = serde_json::from_str(&body)
            .map_err(|e| AppError::Upstream(format!("Failed to parse JSON: {e}")))?;

        if parsed.code != 0 {
            return Err(AppError::from_message(format!(
                "API returned error: code={}, message={}",
                parsed.code, parsed.message
            )));
        }

        let pages = parsed
            .data
            .ok_or_else(|| AppError::VideoNotFound("Video page information not found".into()))?;

        if pages.is_empty() {
            return Err(AppError::VideoNotFound(
                "Video page information not found".into(),
            ));
        }

        pages
            .iter()
            .find(|p| p.page == page)
            .or_else(|| pages.get((page as usize).saturating_sub(1)))
            .map(|p| p.cid)
            .ok_or_else(|| AppError::VideoNotFound(format!("Page {page} not found")))
    }

    pub async fn get_wbi_keys(&self) -> AppResult<WbiKeys> {
        {
            let guard = self.wbi_keys.read().await;
            if let Some(keys) = guard.as_ref() {
                return Ok(keys.clone());
            }
        }

        let mut guard = self.wbi_keys.write().await;
        if let Some(keys) = guard.as_ref() {
            return Ok(keys.clone());
        }

        let url = format!("{BASE_URL}{NAV_ENDPOINT}");
        let req = self.apply_headers(self.http.get(&url), None);
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("Request failed: {e}")))?;
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Upstream(format!("Failed to read response body: {e}")))?;

        let parsed: ApiResponse<NavData> = serde_json::from_str(&body)
            .map_err(|e| AppError::Upstream(format!("Failed to parse JSON: {e}")))?;

        // nav may return code != 0 when not logged in but still include wbi_img
        let data = parsed.data.ok_or_else(|| {
            AppError::Upstream(format!(
                "API returned error: code={}, message={}",
                parsed.code, parsed.message
            ))
        })?;

        let img_key = wbi::extract_key_from_url(&data.wbi_img.img_url);
        let sub_key = wbi::extract_key_from_url(&data.wbi_img.sub_url);
        if img_key.is_empty() || sub_key.is_empty() {
            return Err(AppError::Upstream("Invalid WBI key format".into()));
        }

        let keys = WbiKeys { img_key, sub_key };
        *guard = Some(keys.clone());
        Ok(keys)
    }

    pub async fn get_play_url(&self, bvid: &str, cid: i64, quality: i32) -> AppResult<PlayUrlData> {
        let keys = self.get_wbi_keys().await?;

        let mut params = BTreeMap::new();
        params.insert("bvid".to_string(), bvid.to_string());
        params.insert("cid".to_string(), cid.to_string());
        params.insert("qn".to_string(), quality.to_string());
        params.insert("fnver".to_string(), DEFAULT_FNVER.to_string());
        params.insert("fnval".to_string(), DEFAULT_FNVAL.to_string());
        params.insert("fourk".to_string(), DEFAULT_FOURK.to_string());

        let wts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let signed = wbi::enc_wbi(params, &keys.img_key, &keys.sub_key, wts);
        let query = wbi::build_query_string(&signed);
        let url = format!("{BASE_URL}{PLAYURL_ENDPOINT}?{query}");
        let referer = format!("{VIDEO_URL}/video/{bvid}/");

        let req = self.apply_headers(self.http.get(&url), Some(&referer));
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("Request failed: {e}")))?;
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Upstream(format!("Failed to read response body: {e}")))?;

        let parsed: ApiResponse<PlayUrlData> = serde_json::from_str(&body)
            .map_err(|e| AppError::Upstream(format!("Failed to parse JSON: {e}")))?;

        if parsed.code != 0 {
            return Err(AppError::from_message(format!(
                "API returned error: code={}, message={}",
                parsed.code, parsed.message
            )));
        }

        parsed
            .data
            .ok_or_else(|| AppError::Upstream("Play URL data is empty".into()))
    }
}
