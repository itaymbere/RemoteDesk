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
        if let Ok(out) = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \"$($b.Width) $($b.Height)\"",
            ])
            .output()
        {
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

/// Captura de tela no Windows 10 via ffmpeg (gdigrab) -> VP8 IVF.
pub async fn stream_screen(video_track: Arc<TrackLocalStaticSample>) -> Result<()> {
    info!("Iniciando captura Windows 10 via ffmpeg (gdigrab) -> VP8...");

    let mut child = TokioCommand::new("ffmpeg")
        .args([
            "-f", "gdigrab",
            "-framerate", "30",
            "-i", "desktop",
            "-c:v", "libvpx",
            "-b:v", "1500k",
            "-deadline", "realtime",
            "-cpu-used", "8",
            "-f", "ivf",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("Falha ao iniciar ffmpeg (verifique se está no PATH)")?;

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

/// Injeção de mouse/teclado no Windows 10 via PowerShell (System.Windows.Forms / user32).
pub fn inject_input(event_json: &str) -> Result<()> {
    let evt: InputEvent = serde_json::from_str(event_json)?;
    match evt {
        InputEvent::MouseMove { x, y } => {
            let (w, h) = screen_resolution();
            let px = (x.clamp(0.0, 1.0) * w) as i32;
            let py = (y.clamp(0.0, 1.0) * h) as i32;
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({px}, {py})"
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn();
        }
        InputEvent::MouseDown { button } => {
            let flag = if button == 2 { "0x0008" } else { "0x0002" };
            let script = format!(
                "Add-Type -MemberDefinition '[DllImport(\"user32.dll\")] public static extern void mouse_event(int f, int x, int y, int d, int e);' -Name U32 -Namespace W; [W.U32]::mouse_event({flag},0,0,0,0)"
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn();
        }
        InputEvent::MouseUp { button } => {
            let flag = if button == 2 { "0x0010" } else { "0x0004" };
            let script = format!(
                "Add-Type -MemberDefinition '[DllImport(\"user32.dll\")] public static extern void mouse_event(int f, int x, int y, int d, int e);' -Name U32 -Namespace W; [W.U32]::mouse_event({flag},0,0,0,0)"
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn();
        }
        InputEvent::Wheel { delta_y } => {
            let amount = if delta_y < 0.0 { 120 } else { -120 };
            let script = format!(
                "Add-Type -MemberDefinition '[DllImport(\"user32.dll\")] public static extern void mouse_event(int f, int x, int y, int d, int e);' -Name U32 -Namespace W; [W.U32]::mouse_event(0x0800,0,0,{amount},0)"
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn();
        }
        InputEvent::KeyDown { key } => {
            let escaped = key.replace('\'', "''");
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('{escaped}')"
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn();
        }
        InputEvent::KeyUp { .. } => {}
    }
    Ok(())
}
