use anyhow::Result;
use std::sync::Arc;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub async fn stream_screen(video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    #[cfg(target_os = "linux")]
    return linux::stream_screen(video_track).await;

    #[cfg(target_os = "macos")]
    return macos::stream_screen(video_track).await;

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    anyhow::bail!("Sistema operacional não suportado");
}

pub fn inject_input(event_json: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    return linux::inject_input(event_json);

    #[cfg(target_os = "macos")]
    return macos::inject_input(event_json);

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    anyhow::bail!("Sistema operacional não suportado");
}
