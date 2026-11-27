use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Deserialize;
use std::env;
use std::process::Command;
use std::sync::{Arc, OnceLock};
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

/// Captura de tela no Debian 13 (Wayland via PipeWire/ffmpeg ou X11 via x11grab -> VP8 IVF).
pub async fn stream_screen(video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    let mut cmd = TokioCommand::new("ffmpeg");
    if is_wayland() {
        info!("Iniciando captura Wayland (PipeWire) via ffmpeg -> VP8...");
        cmd.args([
            "-f", "pipewire",
            "-framerate", "30",
            "-i", "0",
        ]);
    } else {
        let x11_display = env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_string());
        info!("Iniciando captura X11 ({}) via ffmpeg -> VP8...", x11_display);
        cmd.args([
            "-f", "x11grab",
            "-framerate", "30",
            "-i", &x11_display,
        ]);
    }

    let mut child = cmd
        .args([
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

fn screen_resolution() -> (f64, f64) {
    static RES: OnceLock<(f64, f64)> = OnceLock::new();
    *RES.get_or_init(|| {
        if let Ok(out) = Command::new("xdotool").arg("getdisplaygeometry").output() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut parts = text.split_whitespace();
            if let (Some(w), Some(h)) = (
                parts.next().and_then(|s| s.parse::<f64>().ok()),
                parts.next().and_then(|s| s.parse::<f64>().ok()),
            ) {
                return (w, h);
            }
        }
        (1920.0, 1080.0)
    })
}

fn map_browser_key(key: &str) -> &str {
    match key {
        "Enter" => "Return",
        "Backspace" => "BackSpace",
        "Escape" => "Escape",
        "Tab" => "Tab",
        " " => "space",
        "Control" => "Control_L",
        "Shift" => "Shift_L",
        "Alt" => "Alt_L",
        "Meta" => "Super_L",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "Delete" => "Delete",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "Page_Up",
        "PageDown" => "Page_Down",
        "CapsLock" => "Caps_Lock",
        "Insert" => "Insert",
        other => other,
    }
}

/// Injeção de mouse/teclado no Debian 13 (Wayland via ydotool/uinput ou X11 via xdotool).
pub fn inject_input(event_json: &str) -> Result<()> {
    let evt: InputEvent = serde_json::from_str(event_json)?;

    if is_wayland() {
        match evt {
            InputEvent::MouseMove { x, y } => {
                let (w, h) = screen_resolution();
                let px = (x.clamp(0.0, 1.0) * w) as i32;
                let py = (y.clamp(0.0, 1.0) * h) as i32;
                let _ = Command::new("ydotool")
                    .args(["mousemove", "--absolute", "-x", &px.to_string(), "-y", &py.to_string()])
                    .spawn();
            }
            InputEvent::MouseDown { button } => {
                let code = match button { 2 => "0x41", _ => "0x40" };
                let _ = Command::new("ydotool").args(["click", code]).spawn();
            }
            InputEvent::MouseUp { .. } => {}
            InputEvent::Wheel { delta_y } => {
                let val = if delta_y < 0.0 { "1" } else { "-1" };
                let _ = Command::new("ydotool").args(["mousemove", "--wheel", "-y", val]).spawn();
            }
            InputEvent::KeyDown { key } => {
                let _ = Command::new("ydotool").args(["type", &key]).spawn();
            }
            InputEvent::KeyUp { .. } => {}
        }
        return Ok(());
    }
    match evt {
        InputEvent::MouseMove { x, y } => {
            let (w, h) = screen_resolution();
            let px = (x.clamp(0.0, 1.0) * w) as i32;
            let py = (y.clamp(0.0, 1.0) * h) as i32;
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
            let mapped = map_browser_key(&key);
            let _ = Command::new("xdotool").args(["keydown", mapped]).spawn();
        }
        InputEvent::KeyUp { key } => {
            let mapped = map_browser_key(&key);
            let _ = Command::new("xdotool").args(["keyup", mapped]).spawn();
        }
    }
    Ok(())
}
