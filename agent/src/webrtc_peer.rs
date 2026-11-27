use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::config::Config;
use crate::platform;
use crate::signaling;

pub struct PeerSession {
    pub pc: Arc<RTCPeerConnection>,
}

pub async fn create_peer_for_offer(
    http: Client,
    cfg: Config,
    client_id: String,
    offer_sdp: Value,
) -> Result<PeerSession> {
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    let mut setting_engine = SettingEngine::default();
    setting_engine.set_include_loopback_candidate(true);

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting_engine)
        .build();

    let rtc_config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let pc = Arc::new(api.new_peer_connection(rtc_config).await?);

    // Track de vídeo VP8 para streaming da tela
    let video_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/vp8".to_owned(),
            ..Default::default()
        },
        "video".to_owned(),
        "ag-remotedesk".to_owned(),
    ));
    pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    // Recebe DataChannel ("input") do navegador para mouse/teclado
    pc.on_data_channel(Box::new(move |dc| {
        let label = dc.label().to_owned();
        Box::pin(async move {
            info!("DataChannel aberto: {label}");
            dc.on_message(Box::new(move |msg| {
                let data = String::from_utf8_lossy(&msg.data).to_string();
                Box::pin(async move {
                    if let Ok(val) = serde_json::from_str::<Value>(&data) {
                        if val.get("type").and_then(|v| v.as_str()) == Some("config") {
                            info!("Nova configuração recebida do servidor: {data}");
                            let _ = std::fs::write("agent_config.json", &data);
                            info!("Reiniciando agente para aplicar configuração...");
                            std::process::exit(0);
                        }
                    }
                    if let Err(e) = platform::inject_input(&data) {
                        warn!("Erro ao injetar input: {e}");
                    }
                })
            }));
        })
    }));

    // Envia ICE candidates para o cliente web
    let http_ice = http.clone();
    let cfg_ice = cfg.clone();
    let target_ice = client_id.clone();
    pc.on_ice_candidate(Box::new(move |cand| {
        let http = http_ice.clone();
        let cfg = cfg_ice.clone();
        let target = target_ice.clone();
        Box::pin(async move {
            if let Some(c) = cand {
                if let Ok(json_cand) = c.to_json() {
                    if let Ok(val) = serde_json::to_value(json_cand) {
                        let _ = signaling::send_signal(&http, &cfg, &target, "ice", &val).await;
                    }
                }
            }
        })
    }));

    pc.on_peer_connection_state_change(Box::new(move |state| {
        info!("Estado da conexão WebRTC: {state}");
        if matches!(
            state,
            RTCPeerConnectionState::Failed
                | RTCPeerConnectionState::Disconnected
                | RTCPeerConnectionState::Closed
        ) {
            warn!("Sessão WebRTC encerrada ({state})");
        }
        Box::pin(async {})
    }));

    let offer: RTCSessionDescription = serde_json::from_value(offer_sdp)?;
    pc.set_remote_description(offer).await?;

    let answer = pc.create_answer(None).await?;
    pc.set_local_description(answer.clone()).await?;

    let answer_val = serde_json::to_value(&answer)?;
    signaling::send_signal(&http, &cfg, &client_id, "answer", &answer_val).await?;
    info!("Answer enviado para {client_id}");

    // Inicia loop de captura de tela em background
    tokio::spawn(async move {
        if let Err(e) = platform::stream_screen(video_track).await {
            warn!("Captura de tela encerrada: {e}");
        }
    });

    Ok(PeerSession { pc })
}

pub async fn add_ice_candidate(pc: &RTCPeerConnection, payload: Value) -> Result<()> {
    let cand: RTCIceCandidateInit = serde_json::from_value(payload)?;
    pc.add_ice_candidate(cand).await?;
    Ok(())
}
