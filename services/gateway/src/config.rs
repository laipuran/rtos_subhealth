//! Process configuration, sourced from CLI flags and environment variables.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub exec_action_name: String,
    pub maps_dir: PathBuf,
    pub db_dir: PathBuf,
    /// When empty, API auth is disabled (development default).
    pub api_token: String,
    /// Directory containing the built WebUI. When absent, only the API is served.
    pub webui_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_port: 5000,
            exec_action_name: "exec_task".into(),
            maps_dir: PathBuf::from("config/maps"),
            db_dir: PathBuf::from("config"),
            api_token: String::new(),
            webui_dir: None,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(port) = std::env::var("GATEWAY_HTTP_PORT") {
            if let Ok(port) = port.parse() {
                cfg.http_port = port;
            }
        }
        if let Ok(name) = std::env::var("GATEWAY_EXEC_ACTION") {
            cfg.exec_action_name = name;
        }
        if let Ok(dir) = std::env::var("GATEWAY_MAPS_DIR") {
            cfg.maps_dir = dir.into();
        }
        if let Ok(dir) = std::env::var("GATEWAY_DB_DIR") {
            cfg.db_dir = dir.into();
        }
        if let Ok(token) = std::env::var("GATEWAY_API_TOKEN") {
            cfg.api_token = token;
        }
        // systemd credentials: prefer the secret mounted by the service manager
        // (see deploy/systemd/gateway.service).
        if let Ok(cred_dir) = std::env::var("CREDENTIALS_DIRECTORY") {
            let path = PathBuf::from(cred_dir).join("api-token");
            if let Ok(token) = std::fs::read_to_string(path) {
                cfg.api_token = token.trim().to_string();
            }
        }
        if let Ok(file) = std::env::var("GATEWAY_API_TOKEN_FILE") {
            if let Ok(token) = std::fs::read_to_string(file) {
                cfg.api_token = token.trim().to_string();
            }
        }
        if let Ok(dir) = std::env::var("GATEWAY_WEBUI_DIR") {
            cfg.webui_dir = Some(dir.into());
        }
        cfg
    }
}
