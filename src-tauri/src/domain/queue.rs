use serde::{Deserialize, Serialize};
use specta::Type;

use super::error::ServiceError;
use super::service::ServiceStartType;

/// An action a queued task executes against a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QueueAction {
    Start,
    Stop,
    Restart,
    ForceStart,
    SetStartType(ServiceStartType),
}

/// Lifecycle of a queued task, as reported to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum QueueTaskStatus {
    Queued,
    Running,
    Success,
    Failed,
}

/// A backend-owned queue task. The backend assigns ids and owns the lifecycle;
/// the frontend renders a projection of these via `QueueTaskUpdated` events.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QueueTask {
    pub id: u32,
    pub service_name: String,
    pub action: QueueAction,
    pub status: QueueTaskStatus,
    /// Structured failure info, present when `status` is `Failed`.
    pub error: Option<ServiceError>,
}
