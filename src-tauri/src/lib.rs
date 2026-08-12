mod commands;
mod contract;
mod domain;
mod liveness;
mod queue;
mod runtime;
mod scm;
mod state;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_appender::non_blocking;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use contract::events::{ServiceConfigChanged, QueueTaskUpdated, ServiceStatusChanged, ServicesChanged};
use domain::error::ServiceError;
use domain::watcher::{NoopServiceWatcher, ServiceWatcher};
use liveness::cache::ServiceCache;
use liveness::service::LivenessService;
use queue::actions::ActionService;
use queue::registry::{TaskEventSink, TaskRegistry};
use runtime::bridge::AsyncServiceRepository;
use scm::windows::{WindowsServiceRepository, WindowsServiceWatcher};
use state::{AppState, TauriEventSink};

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(collect_commands![
            commands::get_services,
            commands::enqueue_action,
            commands::get_queue,
            commands::dismiss_task,
            commands::is_elevated,
            commands::relaunch_as_elevated
        ])
        .events(collect_events![
            ServiceStatusChanged,
            ServiceConfigChanged,
            ServicesChanged,
            QueueTaskUpdated
        ])
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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Ok(dir) = app.path().app_log_dir() {
                std::fs::create_dir_all(&dir)?;
            }
            specta_builder().mount_events(app);

            let cache = Arc::new(RwLock::new(ServiceCache::default()));
            let (signal_tx, signal_rx) = mpsc::channel(256);
            let watcher: Box<dyn ServiceWatcher> = match WindowsServiceWatcher::new(signal_tx) {
                Ok(watcher) => Box::new(watcher),
                Err(error) => {
                    warn!(error = %error, "SCM change subscriptions unavailable; relying on polling");
                    Box::new(NoopServiceWatcher)
                }
            };
            let repository: Arc<AsyncServiceRepository> = Arc::new(AsyncServiceRepository::new(
                Arc::new(WindowsServiceRepository),
            ));
            let (first_refresh_tx, first_refresh_rx) = tokio::sync::watch::channel(
                Err(ServiceError::Internal { message: "initial refresh pending".into() }),
            );
            let sink = Arc::new(TauriEventSink::new(app.handle().clone()));
            let registry = Arc::new(TaskRegistry::new(Arc::clone(&sink) as Arc<dyn TaskEventSink>));
            let liveness = LivenessService::new(
                Arc::clone(&repository),
                watcher,
                Arc::clone(&cache),
                sink,
                first_refresh_tx,
            );
            let liveness_handle = liveness.start(signal_rx);
            app.manage(AppState {
                cache,
                actions: Arc::new(ActionService::new(repository, registry)),
                first_refresh: tokio::sync::Mutex::new(first_refresh_rx),
                _liveness: liveness_handle,
            });
            Ok(())
        })
        .invoke_handler(specta_builder().invoke_handler());

    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(prevent_default_plugin());

    let result = builder.run(tauri::generate_context!());

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

/// Disables WebView2 browser accelerator keys (F5/Ctrl+R reload, Ctrl+F find, Ctrl+J,
/// Ctrl+P print, Ctrl+U source, zoom, ...) natively in release builds. Not registered in
/// debug builds so the dev tools keep their shortcuts.
#[cfg(not(debug_assertions))]
fn prevent_default_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_prevent_default::{Flags, PlatformOptions};

    tauri_plugin_prevent_default::Builder::new()
        .with_flags(Flags::keyboard())
        .platform(PlatformOptions::new().browser_accelerator_keys(false))
        .build()
}
