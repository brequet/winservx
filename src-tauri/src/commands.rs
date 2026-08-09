use tauri::State;
use tracing::{debug, warn};

use crate::{
    domain::error::ServiceError,
    domain::queue::{QueueAction, QueueTask},
    domain::service::ServiceInfo,
    queue::bridge::run_blocking,
    scm::privilege,
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

/// Queues an action against a service; returns the assigned task id immediately.
/// The task runs asynchronously; its lifecycle is reported via
/// `queue-task-updated` events.
#[tauri::command]
#[specta::specta]
pub async fn enqueue_action(
    state: State<'_, AppState>,
    action: QueueAction,
    service_name: String,
) -> Result<u32, ServiceError> {
    debug!(command = "enqueue_action", service = %service_name, action = ?action, "command started");
    Ok(state.actions.enqueue(service_name, action).await)
}

/// Snapshot of the live queue (queued, running and retained failures),
/// ordered by id. The drawer applies it on mount, then patches via events.
#[tauri::command]
#[specta::specta]
pub async fn get_queue(state: State<'_, AppState>) -> Result<Vec<QueueTask>, ServiceError> {
    debug!(command = "get_queue", "command started");
    Ok(state.actions.snapshot().await)
}

/// Removes a failed task the user dismissed; no-op for unknown or in-flight ids.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_task(state: State<'_, AppState>, id: u32) -> Result<(), ServiceError> {
    debug!(command = "dismiss_task", id, "command started");
    if !state.actions.dismiss(id).await {
        warn!(command = "dismiss_task", id, "task unknown or still in flight");
    }
    Ok(())
}
