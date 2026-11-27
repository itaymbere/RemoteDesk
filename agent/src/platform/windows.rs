use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

/// Captura de tela no Windows 10 (DXGI Desktop Duplication) -> stream VP8.
pub async fn stream_screen(_video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!("Iniciando captura de tela (Windows 10)...");
    // TODO: Integrar DXGI Desktop Duplication via crate `windows`
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    Ok(())
}

/// Injeção de mouse/teclado no Windows 10 (SendInput Win32 API).
pub fn inject_input(event_json: &str) -> Result<()> {
    debug!("Input recebido (Windows 10): {event_json}");
    // TODO: Mapear eventos JSON para SendInput
    Ok(())
}
