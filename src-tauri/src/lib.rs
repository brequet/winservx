// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri_specta::{Builder, collect_commands};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GreetResponse {
    pub name: String,
    pub message: String,
}

#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> GreetResponse {
    GreetResponse {
        name: name.to_string(),
        message: format!("Hello, {name}! You've been greeted from Rust!"),
    }
}

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::new().commands(collect_commands![greet])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(specta_builder().invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}