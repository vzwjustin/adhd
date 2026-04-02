use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize file-based logging. Returns a guard that must be held alive
/// for the duration of the program (dropping it flushes remaining logs).
pub fn init(data_dir: &Path) -> WorkerGuard {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    let file_appender = tracing_appender::rolling::daily(&log_dir, "anchor.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("anchor=info")),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    guard
}
