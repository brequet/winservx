use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ServiceKind {
    Win32OwnProcess,
    Win32ShareProcess,
    KernelDriver,
    FileSystemDriver,
    RecognizerDriver,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ServiceStartType {
    Boot,
    System,
    Automatic,
    Manual,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: ServiceState,
    pub start_type: Option<ServiceStartType>,
    pub kind: ServiceKind,
    pub pid: Option<u32>,
    pub binary_path: String,
    /// Account the service runs under (`LocalSystem`, `NT AUTHORITY\NetworkService`, …).
    pub start_name: Option<String>,
}

/// Runtime state of a service, reported by lightweight SCM queries.
/// Used internally by the liveness pipeline; not exported to the frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeStatus {
    pub name: String,
    pub state: ServiceState,
    pub pid: Option<u32>,
}

/// Configuration of a single service, reported by SCM config queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceConfig {
    pub display_name: String,
    pub binary_path: String,
    pub start_type: ServiceStartType,
}
