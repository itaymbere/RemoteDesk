use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

/// Captura de tela no Debian 13 (PipeWire / X11) -> stream VP8.
pub async fn stream_screen(_video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!("Iniciando captura de tela (Debian 13)...");
    // TODO: Integrar captura via PipeWire (Wayland) ou XShm (X11) + encoder VP8
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    Ok(())
}

/// Injeção de mouse/teclado no Debian 13 (uinput / libei / XTest).
pub fn inject_input(event_json: &str) -> Result<()> {
    debug!("Input recebido (Debian 13): {event_json}");
    // TODO: Mapear eventos JSON (mousemove, mousedown, keydown) para uinput/XTest
    Ok(())
}
