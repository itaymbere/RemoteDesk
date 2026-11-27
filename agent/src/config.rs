use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_url: String,
    pub host_id: String,
    pub host_name: String,
    pub host_token: String,
    pub log_file: String,
}

impl Config {
    pub fn default_log_file() -> String {
        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = env::var("HOME") {
                let trimmed = home.trim();
                if !trimmed.is_empty() {
                    return format!("{trimmed}/Library/Logs/RemoteDeskIT/agent.log");
                }
            }
            "/tmp/remotedesk-agent.log".to_string()
        }
        #[cfg(not(target_os = "macos"))]
        {
            env::temp_dir()
                .join("remotedesk-agent.log")
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn from_env() -> Self {
        let server_url = env::var("SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:8080/api.php".to_string());
        let host_id = env::var("HOST_ID").unwrap_or_else(|_| "debian-host-01".to_string());
        let mut host_name = env::var("HOST_NAME").unwrap_or_else(|_| host_id.clone());
        if let Ok(text) = std::fs::read_to_string("agent_config.json") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(name) = val.get("host_name").and_then(|v| v.as_str()) {
                    let trimmed = name.trim();
                    if !trimmed.is_empty() {
                        host_name = trimmed.to_string();
                    }
                }
            }
        }
        let host_token = env::var("HOST_TOKEN").unwrap_or_else(|_| "changeme-token".to_string());
        let log_file = env::var("LOG_FILE")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(Self::default_log_file);

        Self {
            server_url,
            host_id,
            host_name,
            host_token,
            log_file,
        }
    }

    pub fn os_name() -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS 27"
        } else if cfg!(target_os = "linux") {
            "Debian 13"
        } else {
            env::consts::OS
        }
    }
}
