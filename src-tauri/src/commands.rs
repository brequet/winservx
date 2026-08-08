use std::sync::Arc;

use tauri::State;
use tracing::{debug, warn};

use crate::{
    domain::error::ServiceError,
    domain::service::ServiceInfo,
    state::AppState,
};

#[tauri::command]
#[specta::specta]
pub async fn get_services(state: State<'_, AppState>) -> Result<Vec<ServiceInfo>, ServiceError> {
    debug!(command = "get_services", "command started");
    let repository = Arc::clone(&state.repository);
    let result = tauri::async_runtime::spawn_blocking(move || repository.list_services())
        .await
        .map_err(|e| ServiceError::Internal { message: format!("background task panicked: {e}") })
        .and_then(|r| r);
    if let Err(e) = &result {
        warn!(command = "get_services", error = %e, "command failed");
    }
    result
}
