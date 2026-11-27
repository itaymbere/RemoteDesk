use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Deserialize;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info};
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

fn screen_resolution() -> (f64, f64) {
    static RES: OnceLock<(f64, f64)> = OnceLock::new();
    *RES.get_or_init(|| {
        if let Ok(out) = Command::new("osascript")
            .args(["-e", "tell application \"Finder\" to get bounds of window of desktop"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let nums: Vec<f64> = text
                .split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            if nums.len() == 4 {
                return (nums[2], nums[3]);
            }
        }
        (1920.0, 1080.0)
    })
}

/// Captura de tela no macOS 27 Golden Gate via ffmpeg (avfoundation) -> VP8 IVF.
pub async fn stream_screen(video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!("Iniciando captura macOS 27 Golden Gate via ffmpeg (avfoundation) -> VP8...");

    let mut child = TokioCommand::new("ffmpeg")
        .args([
            "-f", "avfoundation",
            "-framerate", "30",
            "-capture_cursor", "1",
            "-i", "1:none",
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
        .context("Falha ao iniciar ffmpeg (instale com: brew install ffmpeg)")?;

    let mut stdout = child.stdout.take().context("Sem stdout do ffmpeg")?;

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

/// Injeção de mouse/teclado no macOS 27 Golden Gate via cliclick + osascript.
pub fn inject_input(event_json: &str) -> Result<()> {
    let evt: InputEvent = serde_json::from_str(event_json)?;
    match evt {
        InputEvent::MouseMove { x, y } => {
            let (w, h) = screen_resolution();
            let px = (x.clamp(0.0, 1.0) * w) as i32;
            let py = (y.clamp(0.0, 1.0) * h) as i32;
            let _ = Command::new("cliclick").arg(format!("m:{px},{py}")).spawn();
        }
        InputEvent::MouseDown { button } => {
            let cmd = if button == 2 { "rc:." } else { "dd:." };
            let _ = Command::new("cliclick").arg(cmd).spawn();
        }
        InputEvent::MouseUp { button } => {
            if button != 2 {
                let _ = Command::new("cliclick").arg("du:.").spawn();
            }
        }
        InputEvent::Wheel { delta_y } => {
            let key_code = if delta_y < 0.0 { "126" } else { "125" };
            let _ = Command::new("osascript")
                .args(["-e", &format!("tell application \"System Events\" to key code {key_code}")])
                .spawn();
        }
        InputEvent::KeyDown { key } => {
            let escaped = key.replace('"', "\\\"");
            let _ = Command::new("osascript")
                .args(["-e", &format!("tell application \"System Events\" to keystroke \"{escaped}\"")])
                .spawn();
        }
        InputEvent::KeyUp { .. } => {}
    }
    Ok(())
}
