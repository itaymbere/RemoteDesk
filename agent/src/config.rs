use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_url: String,
    pub host_id: String,
    pub host_name: String,
    pub host_token: String,
}

impl Config {
    pub fn from_env() -> Self {
        let server_url = env::var("SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:8080/api.php".to_string());
        let host_id = env::var("HOST_ID").unwrap_or_else(|_| "debian-host-01".to_string());
        let host_name = env::var("HOST_NAME").unwrap_or_else(|_| host_id.clone());
        let host_token = env::var("HOST_TOKEN").unwrap_or_else(|_| "changeme-token".to_string());

        Self {
            server_url,
            host_id,
            host_name,
            host_token,
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
