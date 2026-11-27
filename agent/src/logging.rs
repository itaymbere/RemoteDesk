use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// Ensures parent directories exist, rotates `path` to `<path>.1` if it reaches `max_bytes`,
/// and opens the log file in append mode.
pub fn prepare_log_file_with_limit(path: &Path, max_bytes: u64) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    if let Ok(meta) = fs::metadata(path) {
        if meta.len() >= max_bytes {
            let rotated = Path::new(&format!("{}.1", path.display())).to_path_buf();
            let _ = fs::remove_file(&rotated);
            fs::rename(path, &rotated)?;
        }
    }

    OpenOptions::new().create(true).append(true).open(path)
}

#[derive(Clone)]
struct FileMakeWriter {
    file: Arc<Mutex<File>>,
}

struct LockedFileWriter {
    file: Arc<Mutex<File>>,
}

impl Write for LockedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file mutex poisoned"))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file mutex poisoned"))?;
        guard.flush()
    }
}

impl<'a> MakeWriter<'a> for FileMakeWriter {
    type Writer = LockedFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LockedFileWriter {
            file: Arc::clone(&self.file),
        }
    }
}

/// Initializes tracing logs to stdout and optionally to `log_path` (plain text, rotated at 10 MB).
pub fn init_logging(log_path: &str) -> Option<String> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stdout_layer = tracing_subscriber::fmt::layer().with_writer(io::stdout);

    let (file_layer, active_path) = match prepare_log_file_with_limit(
        Path::new(log_path),
        MAX_LOG_SIZE_BYTES,
    ) {
        Ok(file) => {
            let writer = FileMakeWriter {
                file: Arc::new(Mutex::new(file)),
            };
            let layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer);
            (Some(layer), Some(log_path.to_string()))
        }
        Err(err) => {
            eprintln!("Aviso: não foi possível abrir arquivo de log '{log_path}': {err}");
            (None, None)
        }
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    active_path
}

#[cfg(test)]
mod tests {
    use super::prepare_log_file_with_limit;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("remotedesk_log_test_{label}_{nanos}"))
    }

    #[test]
    fn creates_parent_directories_and_appends_logs() {
        let base = unique_test_dir("append");
        let log_path = base.join("nested").join("agent.log");

        {
            let mut file = prepare_log_file_with_limit(&log_path, 1024).unwrap();
            writeln!(file, "first line").unwrap();
        }
        {
            let mut file = prepare_log_file_with_limit(&log_path, 1024).unwrap();
            writeln!(file, "second line").unwrap();
        }

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("first line"));
        assert!(content.contains("second line"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rotates_log_file_when_size_reaches_limit() {
        let base = unique_test_dir("rotate");
        let log_path = base.join("agent.log");
        let rotated_path = base.join("agent.log.1");

        {
            let mut file = prepare_log_file_with_limit(&log_path, 10).unwrap();
            write!(file, "0123456789ab").unwrap();
        }

        {
            let mut file = prepare_log_file_with_limit(&log_path, 10).unwrap();
            writeln!(file, "new").unwrap();
        }

        assert_eq!(fs::read_to_string(&rotated_path).unwrap(), "0123456789ab");
        assert_eq!(fs::read_to_string(&log_path).unwrap(), "new\n");

        let _ = fs::remove_dir_all(&base);
    }
}
