use crate::bilibili::BilibiliClient;
use crate::downloader::Downloader;

#[derive(Clone)]
pub struct AppState {
    pub bilibili: BilibiliClient,
    pub downloader: Downloader,
}

impl AppState {
    pub fn new(cookie: String) -> crate::error::AppResult<Self> {
        let bilibili = BilibiliClient::new(cookie.clone())?;
        let downloader = Downloader::new(bilibili.http().clone(), cookie);
        Ok(Self {
            bilibili,
            downloader,
        })
    }
}
