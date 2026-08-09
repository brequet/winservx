use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::queue::QueueTask;

/// Announces a task state change, carrying the task's full state.
/// Emitted on every lifecycle transition: queued, running, success, failed.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueueTaskUpdated {
    pub task: QueueTask,
}

impl tauri_specta::Event for QueueTaskUpdated {
    const NAME: &'static str = "queue-task-updated";
}
