mod config;
mod platform;
mod signaling;
mod webrtc_peer;

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::webrtc_peer::PeerSession;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = Config::from_env();
    let http = reqwest::Client::new();

    info!(
        "Iniciando RemoteDeskIT Agent | host_id={} | os={}",
        cfg.host_id,
        Config::os_name()
    );

    if let Err(e) = signaling::register_host(&http, &cfg).await {
        warn!("Falha no registro inicial: {e}");
    }

    // Heartbeat a cada 20s para manter o host online no server-php
    let http_hb = http.clone();
    let cfg_hb = cfg.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            interval.tick().await;
            if let Err(e) = signaling::register_host(&http_hb, &cfg_hb).await {
                warn!("Erro no heartbeat: {e}");
            }
        }
    });

    // Loop SSE para receber sinais (offer / ice)
    let (tx, mut rx) = mpsc::channel(64);
    tokio::spawn(signaling::poll_sse_loop(http.clone(), cfg.clone(), tx));

    let mut peers: HashMap<String, PeerSession> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        match msg.signal_type.as_str() {
            "offer" => {
                info!("Offer recebido de {}", msg.sender_id);
                if let Some(old) = peers.remove(&msg.sender_id) {
                    old.close().await;
                }
                match webrtc_peer::create_peer_for_offer(
                    http.clone(),
                    cfg.clone(),
                    msg.sender_id.clone(),
                    msg.payload,
                )
                .await
                {
                    Ok(session) => {
                        peers.insert(msg.sender_id, session);
                    }
                    Err(e) => error!("Erro ao criar PeerConnection: {e}"),
                }
            }
            "ice" => {
                if let Some(session) = peers.get(&msg.sender_id) {
                    if let Err(e) = webrtc_peer::add_ice_candidate(&session.pc, msg.payload).await {
                        warn!("Erro ao adicionar ICE candidate: {e}");
                    }
                }
            }
            other => {
                warn!("Sinal desconhecido: {other}");
            }
        }
    }
}
