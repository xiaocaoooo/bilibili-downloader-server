use std::env;

use crate::error::AppError;

const DEFAULT_PORT: u16 = 8080;
const ENV_COOKIE: &str = "BILIBILI_COOKIE";
const ENV_PORT: &str = "PORT";

#[derive(Debug, Clone)]
pub struct Config {
    pub cookie: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let cookie = env::var(ENV_COOKIE).map_err(|_| {
            AppError::Internal(format!("Environment variable {ENV_COOKIE} must be set"))
        })?;

        if cookie.trim().is_empty() {
            return Err(AppError::Internal(format!(
                "Environment variable {ENV_COOKIE} must not be empty"
            )));
        }

        let port = match env::var(ENV_PORT) {
            Ok(value) if !value.is_empty() => value
                .parse::<u16>()
                .map_err(|_| AppError::Internal(format!("Invalid {ENV_PORT} value: {value}")))?,
            _ => DEFAULT_PORT,
        };

        Ok(Self { cookie, port })
    }
}
