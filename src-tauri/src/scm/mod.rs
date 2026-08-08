pub mod windows;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::service::ServiceInfo;

/// Error returned by SCM operations, serialized to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Type, thiserror::Error)]
#[serde(rename_all = "camelCase")]
pub enum ScmError {
    #[error("Windows API error: {0}")]
    Windows(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<::windows::core::Error> for ScmError {
    fn from(value: ::windows::core::Error) -> Self {
        ScmError::Windows(value.message())
    }
}

/// Port that the rest of the app depends on. Implemented by platform SCM adapters.
pub trait ServiceRepository: Send + Sync {
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ScmError>;
}

pub type DynServiceRepository = Arc<dyn ServiceRepository>;
