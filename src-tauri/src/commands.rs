use std::sync::Arc;

use tauri::State;

use crate::{
    domain::service::ServiceInfo,
    scm::ScmError,
    state::AppState,
};

#[tauri::command]
#[specta::specta]
pub async fn get_services(state: State<'_, AppState>) -> Result<Vec<ServiceInfo>, ScmError> {
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || repository.list_services())
        .await
        .map_err(|e| ScmError::Internal(format!("background task panicked: {e}")))?
}
