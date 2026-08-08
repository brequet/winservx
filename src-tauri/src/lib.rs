mod commands;
mod domain;
mod scm;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands};
use tracing::{error, info};
use tracing_appender::non_blocking;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use scm::windows::WindowsServiceRepository;
use state::AppState;

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(collect_commands![commands::get_services])
}

pub fn run() {
    install_panic_hook();

    // RUST_LOG if set, otherwise info (debug builds) / warn (release builds).
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if cfg!(debug_assertions) { "info" } else { "warn" })
    });

    // Tauri resolves app_log_dir() as {app_data_dir}/logs, i.e. %APPDATA%\fr.requet.winservx\logs
    // on Windows. The app handle is not available before the builder runs, so resolve the
    // same path from environment variables; the .setup() hook below re-resolves it through
    // Tauri's path API and creates the directory. Fall back to the current directory.
    let log_dir = env_log_dir().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
    let _ = std::fs::create_dir_all(&log_dir);

    // stdout (ANSI in debug, plain in release) plus a non-blocking rolling file appender,
    // so a slow disk can never stall the event loop.
    let file_appender = tracing_appender::rolling::never(&log_dir, "winservx.log");
    let (file_writer, _file_guard) = non_blocking(file_appender);
    let writer = (|| std::io::stdout()).and(file_writer);

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(cfg!(debug_assertions))
        .try_init();

    let log_file = log_dir.join("winservx.log");
    info!(
        app = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        release = !cfg!(debug_assertions),
        log_file = %log_file.display(),
        "winservx starting"
    );

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            repository: Arc::new(WindowsServiceRepository),
        })
        .setup(|app| {
            if let Ok(dir) = app.path().app_log_dir() {
                std::fs::create_dir_all(&dir)?;
            }
            Ok(())
        })
        .invoke_handler(specta_builder().invoke_handler())
        .run(tauri::generate_context!());

    if let Err(e) = result {
        error!("application failed to run: {e}");
        std::process::exit(1);
    }
}

/// Logs panics at `error` level with payload and location, alongside the default hook.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        match panic_info.location() {
            Some(loc) => {
                error!(
                    panic = %message,
                    file = loc.file(),
                    line = loc.line(),
                    column = loc.column(),
                    "thread panicked"
                );
            }
            None => {
                error!(panic = %message, "thread panicked");
            }
        }
    }));
}

/// Best-effort replication of Tauri's `app_log_dir()` (%APPDATA%\<identifier>\logs) without
/// an app handle, used to place the log file before the builder runs.
fn env_log_dir() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(app_data).join("fr.requet.winservx").join("logs"))
}
