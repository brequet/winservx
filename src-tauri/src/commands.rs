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
