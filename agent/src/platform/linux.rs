use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Deserialize;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info, warn};
use webrtc::media::Sample;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum InputEvent {
    #[serde(rename = "mousemove")]
    MouseMove { x: f64, y: f64 },
    #[serde(rename = "mousedown")]
    MouseDown { button: u8 },
    #[serde(rename = "mouseup")]
    MouseUp { button: u8 },
    #[serde(rename = "wheel")]
    Wheel { #[serde(rename = "deltaY")] delta_y: f64 },
    #[serde(rename = "keydown")]
    KeyDown { key: String },
    #[serde(rename = "keyup")]
    KeyUp { key: String },
}

fn is_wayland() -> bool {
    env::var("XDG_SESSION_TYPE")
        .map(|v| v.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

/// Captura de tela no Debian 13 (X11 via ffmpeg x11grab -> VP8 IVF; gancho para Wayland).
pub async fn stream_screen(video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    if is_wayland() {
        warn!("Sessão Wayland detectada: captura via PipeWire ainda não implementada.");
        tokio::time::sleep(Duration::from_secs(3600)).await;
        return Ok(());
    }

    let x11_display = env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_string());
    info!("Iniciando captura X11 ({}) via ffmpeg -> VP8...", x11_display);

    let mut child = TokioCommand::new("ffmpeg")
        .args([
            "-f", "x11grab",
            "-framerate", "30",
            "-i", &x11_display,
            "-c:v", "libvpx",
            "-b:v", "1500k",
            "-deadline", "realtime",
            "-cpu-used", "8",
            "-f", "ivf",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Falha ao iniciar ffmpeg (verifique se está instalado)")?;

    let mut stdout = child.stdout.take().context("Sem stdout do ffmpeg")?;

    // Pula cabeçalho IVF de 32 bytes
    let mut ivf_header = [0u8; 32];
    stdout.read_exact(&mut ivf_header).await?;

    let mut frame_hdr = [0u8; 12];
    while stdout.read_exact(&mut frame_hdr).await.is_ok() {
        let frame_size = u32::from_le_bytes([
            frame_hdr[0],
            frame_hdr[1],
            frame_hdr[2],
            frame_hdr[3],
        ]) as usize;

        let mut frame_data = vec![0u8; frame_size];
        if stdout.read_exact(&mut frame_data).await.is_err() {
            break;
        }

        let sample = Sample {
            data: Bytes::from(frame_data),
            duration: Duration::from_millis(33),
            ..Default::default()
        };

        if let Err(e) = video_track.write_sample(&sample).await {
            debug!("Erro ao enviar sample VP8: {e}");
            break;
        }
    }

    let _ = child.kill().await;
    Ok(())
}

/// Injeção de mouse/teclado no Debian 13 (X11 via xdotool; gancho para Wayland).
pub fn inject_input(event_json: &str) -> Result<()> {
    if is_wayland() {
        return Ok(());
    }

    let evt: InputEvent = serde_json::from_str(event_json)?;
    match evt {
        InputEvent::MouseMove { x, y } => {
            // Coordenadas normalizadas (0..1) -> assume tela 1920x1080 se não consultada
            let px = (x.clamp(0.0, 1.0) * 1920.0) as i32;
            let py = (y.clamp(0.0, 1.0) * 1080.0) as i32;
            let _ = Command::new("xdotool")
                .args(["mousemove", &px.to_string(), &py.to_string()])
                .spawn();
        }
        InputEvent::MouseDown { button } => {
            let btn = (button + 1).to_string();
            let _ = Command::new("xdotool").args(["mousedown", &btn]).spawn();
        }
        InputEvent::MouseUp { button } => {
            let btn = (button + 1).to_string();
            let _ = Command::new("xdotool").args(["mouseup", &btn]).spawn();
        }
        InputEvent::Wheel { delta_y } => {
            let btn = if delta_y < 0.0 { "4" } else { "5" };
            let _ = Command::new("xdotool").args(["click", btn]).spawn();
        }
        InputEvent::KeyDown { key } => {
            let _ = Command::new("xdotool").args(["keydown", &key]).spawn();
        }
        InputEvent::KeyUp { key } => {
            let _ = Command::new("xdotool").args(["keyup", &key]).spawn();
        }
    }
    Ok(())
}
