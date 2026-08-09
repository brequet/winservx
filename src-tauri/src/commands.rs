use std::sync::Arc;

use tauri::State;
use tracing::{debug, warn};

use crate::{
    domain::error::ServiceError,
    domain::service::{ServiceInfo, ServiceStartType},
    privilege,
    state::AppState,
};

#[tauri::command]
#[specta::specta]
pub fn is_elevated() -> bool {
    privilege::is_elevated()
}

/// Relaunches the app with a UAC prompt; the current process exits on success.
#[tauri::command]
#[specta::specta]
pub async fn relaunch_as_elevated(app: tauri::AppHandle) -> Result<(), ServiceError> {
    let result = tauri::async_runtime::spawn_blocking(privilege::relaunch_elevated)
        .await
        .map_err(|e| ServiceError::Internal { message: format!("relaunch task panicked: {e}") })?;
    if result? == privilege::RelaunchOutcome::Launched {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_services(state: State<'_, AppState>) -> Result<Vec<ServiceInfo>, ServiceError> {
    debug!(command = "get_services", "command started");
    {
        let cache = state.cache.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !cache.is_empty() {
            return Ok(cache.snapshot());
        }
    }

    // The liveness pipeline has not produced its first snapshot yet; query the
    // SCM directly and seed the cache so the frontend gets an immediate result.
    let repository = Arc::clone(&state.repository);
    let cache = Arc::clone(&state.cache);
    let result = tauri::async_runtime::spawn_blocking(move || repository.list_services())
        .await
        .map_err(|e| ServiceError::Internal { message: format!("background task panicked: {e}") })
        .and_then(|r| r);
    if let Ok(ref services) = result {
        let mut cache = cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if cache.is_empty() {
            cache.apply_full_snapshot(services.clone());
        }
    }
    if let Err(e) = &result {
        warn!(command = "get_services", error = %e, "command failed");
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn start_service(state: State<'_, AppState>, name: String) -> Result<(), ServiceError> {
    debug!(command = "start_service", service = %name, "command started");
    let result = state.actions.start(&name).await;
    log_action_result("start_service", &name, &result);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn stop_service(state: State<'_, AppState>, name: String) -> Result<(), ServiceError> {
    debug!(command = "stop_service", service = %name, "command started");
    let result = state.actions.stop(&name).await;
    log_action_result("stop_service", &name, &result);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn restart_service(state: State<'_, AppState>, name: String) -> Result<(), ServiceError> {
    debug!(command = "restart_service", service = %name, "command started");
    let result = state.actions.restart(&name).await;
    log_action_result("restart_service", &name, &result);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn force_start_service(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), ServiceError> {
    debug!(command = "force_start_service", service = %name, "command started");
    let result = state.actions.force_start(&name).await;
    log_action_result("force_start_service", &name, &result);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn update_startup_type(
    state: State<'_, AppState>,
    name: String,
    start_type: ServiceStartType,
) -> Result<(), ServiceError> {
    debug!(command = "update_startup_type", service = %name, start_type = ?start_type, "command started");
    let result = state.actions.set_start_type(&name, start_type).await;
    log_action_result("update_startup_type", &name, &result);
    result
}

fn log_action_result(command: &str, service: &str, result: &Result<(), ServiceError>) {
    match result {
        Ok(()) => debug!(command, service, "command succeeded"),
        Err(error) => warn!(command, service, error = %error, "command failed"),
    }
}
