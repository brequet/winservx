mod commands;
mod domain;
mod scm;
mod state;

use std::sync::Arc;

use tauri_specta::{Builder, collect_commands};

use scm::windows::WindowsServiceRepository;
use state::AppState;

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(collect_commands![commands::get_services])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            repository: Arc::new(WindowsServiceRepository),
        })
        .invoke_handler(specta_builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
