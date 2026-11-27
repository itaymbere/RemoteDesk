use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct SignalMessage {
    pub id: u64,
    pub sender_id: String,
    #[serde(rename = "type")]
    pub signal_type: String,
    pub payload: Value,
}

#[derive(Serialize)]
struct RegisterPayload<'a> {
    host_id: &'a str,
    name: &'a str,
    os: &'a str,
    token: &'a str,
}

#[derive(Serialize)]
struct SendSignalPayload<'a> {
    host_id: &'a str,
    token: &'a str,
    target_id: &'a str,
    #[serde(rename = "type")]
    signal_type: &'a str,
    payload: &'a Value,
}

pub async fn register_host(client: &Client, cfg: &Config) -> Result<()> {
    let url = format!("{}?action=host_register", cfg.server_url);
    let body = RegisterPayload {
        host_id: &cfg.host_id,
        name: &cfg.host_name,
        os: Config::os_name(),
        token: &cfg.host_token,
    };

    client.post(&url).json(&body).send().await?.error_for_status()?;
    debug!("Host registrado/heartbeat enviado");
    Ok(())
}

pub async fn send_signal(
    client: &Client,
    cfg: &Config,
    target_id: &str,
    signal_type: &str,
    payload: &Value,
) -> Result<()> {
    let url = format!("{}?action=signal", cfg.server_url);
    let body = SendSignalPayload {
        host_id: &cfg.host_id,
        token: &cfg.host_token,
        target_id,
        signal_type,
        payload,
    };

    client.post(&url).json(&body).send().await?.error_for_status()?;
    Ok(())
}

pub async fn poll_sse_loop(
    client: Client,
    cfg: Config,
    tx: mpsc::Sender<SignalMessage>,
) {
    let mut last_id: u64 = 0;

    loop {
        let url = format!(
            "{}?action=poll&host_id={}&token={}&last_id={}",
            cfg.server_url, cfg.host_id, cfg.host_token, last_id
        );

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                            while let Some(pos) = buffer.find("\n\n") {
                                let event_block = buffer[..pos].to_string();
                                buffer = buffer[pos + 2..].to_string();

                                for line in event_block.lines() {
                                    if let Some(json_str) = line.strip_prefix("data: ") {
                                        if let Ok(msg) = serde_json::from_str::<SignalMessage>(json_str) {
                                            last_id = msg.id.max(last_id);
                                            if tx.send(msg).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Erro no stream SSE: {e}");
                            break;
                        }
                    }
                }
            }
            Ok(resp) => {
                warn!("SSE retornou status {}", resp.status());
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                warn!("Falha ao conectar no SSE: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
