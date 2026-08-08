use std::sync::Arc;

use crate::domain::{
    error::ServiceError,
    service::{ServiceConfig, ServiceInfo, ServiceRuntimeStatus},
};

/// Port that the rest of the app depends on. Implemented by platform SCM adapters.
pub trait ServiceRepository: Send + Sync {
    /// Full snapshot including configuration, for initial load and full refreshes.
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError>;
    /// Status-only snapshot from a single enumeration call, for the reconciliation poll.
    fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError>;
    /// Queries a single service's runtime status; `Ok(None)` when it no longer exists.
    fn query_service_status(&self, name: &str) -> Result<Option<ServiceRuntimeStatus>, ServiceError>;
    /// Queries a single service's configuration; `Ok(None)` when it no longer exists.
    fn query_config(&self, name: &str) -> Result<Option<ServiceConfig>, ServiceError>;
}

pub type DynServiceRepository = Arc<dyn ServiceRepository>;
