use tauri::State;
use tracing::{debug, warn};

use crate::{
    domain::error::ServiceError,
    domain::service::{ServiceInfo, ServiceStartType},
    privilege,
    queue::bridge::run_blocking,
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
    let outcome = run_blocking(privilege::relaunch_elevated).await??;
    if outcome == privilege::RelaunchOutcome::Launched {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_services(state: State<'_, AppState>) -> Result<Vec<ServiceInfo>, ServiceError> {
    debug!(command = "get_services", "command started");
    // Wait for the liveness pipeline's first refresh; the cache is fully
    // written before the signal flips. Later calls take the current outcome.
    let outcome = {
        let mut first_refresh = state.first_refresh.lock().await;
        if first_refresh.has_changed().unwrap_or(false) {
            first_refresh.borrow().clone()
        } else {
            first_refresh
                .changed()
                .await
                .map_err(|_| ServiceError::Internal { message: "liveness pipeline stopped".into() })?;
            first_refresh.borrow().clone()
        }
    };
    if let Err(error) = outcome {
        warn!(command = "get_services", error = %error, "initial refresh failed");
        return Err(error);
    }
    let cache = state.cache.read().unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(cache.snapshot())
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
