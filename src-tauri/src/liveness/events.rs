use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::service::{ServiceInfo, ServiceStartType, ServiceState};

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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServicesChanged {
    pub added: Vec<ServiceInfo>,
    pub removed: Vec<String>,
}

impl tauri_specta::Event for ServicesChanged {
    const NAME: &'static str = "services-changed";
}

/// Events produced by the liveness pipeline, routed to the frontend sink.
#[derive(Debug, Clone)]
pub enum LivenessEvent {
    Status(ServiceStatusChanged),
    Config(ServiceConfigChanged),
    Services(ServicesChanged),
}
