use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::queue::QueueTask;
use crate::domain::service::{ServiceInfo, ServiceStartType, ServiceState};

/// Announces that a service's runtime status changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatusChanged {
    pub name: String,
    pub state: ServiceState,
    pub pid: Option<u32>,
}

impl tauri_specta::Event for ServiceStatusChanged {
    const NAME: &'static str = "service-status-changed";
}

/// Announces that a service's configuration changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfigChanged {
    pub name: String,
    pub display_name: String,
    pub start_type: ServiceStartType,
}

impl tauri_specta::Event for ServiceConfigChanged {
    const NAME: &'static str = "service-config-changed";
}

/// Announces services added to or removed from the service database.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServicesChanged {
    pub added: Vec<ServiceInfo>,
    pub removed: Vec<String>,
}

impl tauri_specta::Event for ServicesChanged {
    const NAME: &'static str = "services-changed";
}

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
