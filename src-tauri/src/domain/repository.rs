use std::sync::Arc;

use crate::domain::{error::ServiceError, service::ServiceInfo};

/// Port that the rest of the app depends on. Implemented by platform SCM adapters.
pub trait ServiceRepository: Send + Sync {
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError>;
}

pub type DynServiceRepository = Arc<dyn ServiceRepository>;
