use anyhow::{Context, Result};
use bytes::Bytes;
use serde::Deserialize;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
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
    KeyUp {
        #[allow(dead_code)]
        key: String,
    },
}

#[derive(Debug, Deserialize)]
struct AgentVideoConfig {
    #[serde(default = "default_fps")]
    fps: u64,
    #[serde(default = "default_bitrate")]
    bitrate: String,
}

fn default_fps() -> u64 {
    15
}

fn default_bitrate() -> String {
    "1500k".to_string()
}

fn load_video_config() -> AgentVideoConfig {
    if let Ok(text) = std::fs::read_to_string("agent_config.json") {
        if let Ok(cfg) = serde_json::from_str::<AgentVideoConfig>(&text) {
            return cfg;
        }
    }
    AgentVideoConfig {
        fps: default_fps(),
        bitrate: default_bitrate(),
    }
}

fn extract_avfoundation_index(line: &str) -> Option<u32> {
    let marker_pos = line.find("Capture screen")?;
    let prefix = line[..marker_pos].trim_end();
    let open = prefix.rfind('[')?;
    let close = prefix.rfind(']')?;
    if close > open && close + 1 == prefix.len() {
        prefix[open + 1..close].trim().parse::<u32>().ok()
    } else {
        None
    }
}

/// Extracts the AVFoundation screen capture device (`<idx>:none`) from `ffmpeg -list_devices` output.
fn parse_avfoundation_screen_device(output: &str) -> Option<String> {
    let mut in_video_section = false;
    let mut fallback: Option<u32> = None;

    for line in output.lines() {
        if line.contains("AVFoundation video devices:") {
            in_video_section = true;
            continue;
        }
        if line.contains("AVFoundation audio devices:") {
            if in_video_section {
                break;
            }
            continue;
        }
        if !in_video_section {
            continue;
        }
        if let Some(idx) = extract_avfoundation_index(line) {
            if line.contains("Capture screen 0") {
                return Some(format!("{idx}:none"));
            }
            if fallback.is_none() {
                fallback = Some(idx);
            }
        }
    }

    fallback.map(|idx| format!("{idx}:none"))
}

/// Resolves the AVFoundation input device via `AVFOUNDATION_DEVICE`, auto-detection, or `"1:none"` fallback.
fn detect_avfoundation_device() -> String {
    if let Ok(val) = std::env::var("AVFOUNDATION_DEVICE") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Ok(out) = Command::new("ffmpeg")
        .args(["-f", "avfoundation", "-list_devices", "true", "-i", ""])
        .output()
    {
        let mut combined = String::from_utf8_lossy(&out.stderr).into_owned();
        combined.push('\n');
        combined.push_str(&String::from_utf8_lossy(&out.stdout));
        if let Some(dev) = parse_avfoundation_screen_device(&combined) {
            return dev;
        }
    }

    "1:none".to_string()
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
    let vcfg = load_video_config();
    let fps_clamped = vcfg.fps.clamp(1, 60);
    let fps_str = fps_clamped.to_string();
    let gop_str = (fps_clamped * 2).max(10).to_string();
    let max_width: u32 = std::env::var("MAX_WIDTH")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|&w| w >= 640)
        .unwrap_or(1920);
    let vf = format!("fps={fps_clamped},scale='min({max_width},iw)':-2:flags=lanczos");
    let frame_duration = Duration::from_millis(1000 / fps_clamped);
    let device = detect_avfoundation_device();

    info!(
        "Iniciando captura macOS 27 Golden Gate via ffmpeg (avfoundation, device={}, {}fps, {}, max_w={}) -> VP8...",
        device, fps_str, vcfg.bitrate, max_width
    );

    let mut child = TokioCommand::new("ffmpeg")
        .args([
            "-loglevel", "warning",
            "-f", "avfoundation",
            "-framerate", "30",
            "-capture_cursor", "1",
            "-pixel_format", "nv12",
            "-i", &device,
            "-vf", &vf,
            "-r", &fps_str,
            "-pix_fmt", "yuv420p",
            "-c:v", "libvpx",
            "-b:v", &vcfg.bitrate,
            "-maxrate", &vcfg.bitrate,
            "-bufsize", &vcfg.bitrate,
            "-qmin", "4",
            "-qmax", "30",
            "-g", &gop_str,
            "-keyint_min", &fps_str,
            "-lag-in-frames", "0",
            "-error-resilient", "1",
            "-deadline", "realtime",
            "-cpu-used", "6",
            "-threads", "4",
            "-f", "ivf",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("Falha ao iniciar ffmpeg (instale com: brew install ffmpeg)")?;

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    warn!("[ffmpeg] {trimmed}");
                }
            }
        });
    }

    let mut stdout = child.stdout.take().context("Sem stdout do ffmpeg")?;

    let mut ivf_header = [0u8; 32];
    stdout.read_exact(&mut ivf_header).await?;

    let mut frame_hdr = [0u8; 12];
    let mut sent_first_frame = false;
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
            duration: frame_duration,
            ..Default::default()
        };

        if let Err(e) = video_track.write_sample(&sample).await {
            debug!("Erro ao enviar sample VP8: {e}");
            break;
        }
        if !sent_first_frame {
            sent_first_frame = true;
            info!("Primeiro frame VP8 enviado ({} bytes)", frame_size);
        }
    }

    let _ = child.kill().await;
    Ok(())
}

fn spawn_logged(cmd: &mut Command, tool: &str) {
    if let Err(e) = cmd.spawn() {
        warn!("Falha ao executar {tool}: {e}");
    }
}

/// Injeção de mouse/teclado no macOS 27 Golden Gate via cliclick + osascript.
pub fn inject_input(event_json: &str) -> Result<()> {
    let evt: InputEvent = serde_json::from_str(event_json)?;
    match evt {
        InputEvent::MouseMove { x, y } => {
            let (w, h) = screen_resolution();
            let px = (x.clamp(0.0, 1.0) * w) as i32;
            let py = (y.clamp(0.0, 1.0) * h) as i32;
            spawn_logged(Command::new("cliclick").arg(format!("m:{px},{py}")), "cliclick");
        }
        InputEvent::MouseDown { button } => {
            let cmd = if button == 2 { "rc:." } else { "dd:." };
            spawn_logged(Command::new("cliclick").arg(cmd), "cliclick");
        }
        InputEvent::MouseUp { button } => {
            if button != 2 {
                spawn_logged(Command::new("cliclick").arg("du:."), "cliclick");
            }
        }
        InputEvent::Wheel { delta_y } => {
            let key_code = if delta_y < 0.0 { "126" } else { "125" };
            spawn_logged(
                Command::new("osascript")
                    .args(["-e", &format!("tell application \"System Events\" to key code {key_code}")]),
                "osascript",
            );
        }
        InputEvent::KeyDown { key } => {
            let escaped = key.replace('"', "\\\"");
            spawn_logged(
                Command::new("osascript")
                    .args(["-e", &format!("tell application \"System Events\" to keystroke \"{escaped}\"")]),
                "osascript",
            );
        }
        InputEvent::KeyUp { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_avfoundation_screen_device;

    #[test]
    fn detects_capture_screen_0_from_real_ffmpeg_output() {
        let sample = "\
[AVFoundation indev @ 0x7b1d018180] AVFoundation video devices:
[AVFoundation indev @ 0x7b1d018180] [0] Câmera FaceTime HD  [uid:EAB7A68F-EC2B-4487-AADF-D8A91C1CB782]
[AVFoundation indev @ 0x7b1d018180] [1] OBS Virtual Camera  [uid:7626645E-4425-469E-9D8B-97E0FA59AC75]
[AVFoundation indev @ 0x7b1d018180] [2] Câmera do iPhone Rod  [uid:240F70AA-4D6D-4777-B3A4-8D3800000001]
[AVFoundation indev @ 0x7b1d018180] [3] Câmera da Visualização da Mesa iPhone Rod  [uid:240F70AA-4D6D-4777-B3A4-8D3800000002]
[AVFoundation indev @ 0x7b1d018180] [4] Capture screen 0
[AVFoundation indev @ 0x7b1d018180] AVFoundation audio devices:
[AVFoundation indev @ 0x7b1d018180] [0] Microfone (MacBook Pro)  [uid:BuiltInMicrophoneDevice]";

        assert_eq!(
            parse_avfoundation_screen_device(sample),
            Some("4:none".to_string())
        );
    }

    #[test]
    fn prefers_capture_screen_0_over_capture_screen_1() {
        let sample = "\
[AVFoundation indev @ 0x1] AVFoundation video devices:
[AVFoundation indev @ 0x1] [2] Capture screen 1
[AVFoundation indev @ 0x1] [3] Capture screen 0
[AVFoundation indev @ 0x1] AVFoundation audio devices:";

        assert_eq!(
            parse_avfoundation_screen_device(sample),
            Some("3:none".to_string())
        );
    }

    #[test]
    fn ignores_audio_section_and_returns_none_without_screen_device() {
        let sample = "\
[AVFoundation indev @ 0x1] AVFoundation video devices:
[AVFoundation indev @ 0x1] [0] Câmera FaceTime HD
[AVFoundation indev @ 0x1] AVFoundation audio devices:
[AVFoundation indev @ 0x1] [0] Capture screen 0";

        assert_eq!(parse_avfoundation_screen_device(sample), None);
    }
}
