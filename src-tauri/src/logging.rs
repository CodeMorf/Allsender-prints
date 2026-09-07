use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init() -> Result<(), String> {
    let dir = crate::config::logs_dir()?;
    let file_appender = tracing_appender::rolling::daily(dir, "restaapp-printer.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .try_init()
        .map_err(|e| e.to_string())?;
    tracing::info!("RestaAPP Printer {} iniciado", crate::models::APP_VERSION);
    Ok(())
}
