use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

/// Captura de tela no macOS 27 Golden Gate (ScreenCaptureKit) -> stream VP8.
pub async fn stream_screen(_video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!("Iniciando captura de tela (macOS 27 Golden Gate)...");
    // TODO: Integrar ScreenCaptureKit + VideoToolbox/VP8
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    Ok(())
}

/// Injeção de mouse/teclado no macOS 27 Golden Gate (CGEvent).
pub fn inject_input(event_json: &str) -> Result<()> {
    debug!("Input recebido (macOS 27): {event_json}");
    // TODO: Mapear eventos JSON para CGEvent (requer permissão de Acessibilidade)
    Ok(())
}
