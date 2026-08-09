use std::sync::Arc;

use crate::domain::{
    error::ServiceError,
    repository::DynServiceRepository,
    service::{ServiceConfig, ServiceInfo, ServiceRuntimeStatus, ServiceStartType},
};

/// Runs a blocking closure on the async runtime's blocking thread pool.
///
/// A panic in the closure surfaces as `ServiceError::Internal`; callers only
/// ever see the closure's own `Result`. The single place where threading and
/// panic behavior live.
pub async fn run_blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ServiceError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ServiceError::Internal { message: format!("blocking task panicked: {e}") })
}

/// Async facade over the synchronous `ServiceRepository` port.
///
/// Bridges blocking SCM calls onto the async runtime's thread pool so async
/// consumers (actions, liveness, commands) never deal with threads or panics
/// themselves.
#[derive(Clone)]
pub struct AsyncServiceRepository {
    inner: DynServiceRepository,
}

impl AsyncServiceRepository {
    pub fn new(inner: DynServiceRepository) -> Self {
        Self { inner }
    }

    pub async fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.list_services()).await?
    }

    pub async fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.list_states()).await?
    }

    pub async fn query_service_status(
        &self,
        name: &str,
    ) -> Result<Option<ServiceRuntimeStatus>, ServiceError> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        run_blocking(move || inner.query_service_status(&name)).await?
    }

    pub async fn query_config(&self, name: &str) -> Result<Option<ServiceConfig>, ServiceError> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        run_blocking(move || inner.query_config(&name)).await?
    }

    pub async fn start_service(&self, name: &str) -> Result<(), ServiceError> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        run_blocking(move || inner.start_service(&name)).await?
    }

    pub async fn stop_service(&self, name: &str) -> Result<(), ServiceError> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        run_blocking(move || inner.stop_service(&name)).await?
    }

    pub async fn set_start_type(
        &self,
        name: &str,
        start_type: ServiceStartType,
    ) -> Result<(), ServiceError> {
        let inner = Arc::clone(&self.inner);
        let name = name.to_owned();
        run_blocking(move || inner.set_start_type(&name, start_type)).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::ServiceRepository;
    use crate::domain::service::ServiceState;

    struct StubRepository {
        start_result: Result<(), ServiceError>,
        start_panics: bool,
        status: Option<ServiceRuntimeStatus>,
    }

    impl ServiceRepository for StubRepository {
        fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
            Ok(Vec::new())
        }

        fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError> {
            Ok(Vec::new())
        }

        fn query_service_status(
            &self,
            name: &str,
        ) -> Result<Option<ServiceRuntimeStatus>, ServiceError> {
            Ok(self.status.clone().map(|mut status| {
                status.name = name.to_owned();
                status
            }))
        }

        fn query_config(&self, _name: &str) -> Result<Option<ServiceConfig>, ServiceError> {
            Ok(None)
        }

        fn start_service(&self, _name: &str) -> Result<(), ServiceError> {
            if self.start_panics {
                panic!("mock start panicked");
            }
            self.start_result.clone()
        }

        fn stop_service(&self, _name: &str) -> Result<(), ServiceError> {
            Ok(())
        }

        fn set_start_type(
            &self,
            _name: &str,
            _start_type: ServiceStartType,
        ) -> Result<(), ServiceError> {
            Ok(())
        }
    }

    fn test_bridge(repository: StubRepository) -> AsyncServiceRepository {
        AsyncServiceRepository::new(Arc::new(repository))
    }

    #[tokio::test]
    async fn passes_values_through() {
        let repository = test_bridge(StubRepository {
            start_result: Ok(()),
            start_panics: false,
            status: Some(ServiceRuntimeStatus {
                name: String::new(),
                state: ServiceState::Running,
                pid: Some(42),
            }),
        });
        let status = repository.query_service_status("svc").await.unwrap().unwrap();
        assert_eq!(status.name, "svc");
        assert_eq!(status.state, ServiceState::Running);
        assert_eq!(status.pid, Some(42));
    }

    #[tokio::test]
    async fn passes_errors_through() {
        let repository = test_bridge(StubRepository {
            start_result: Err(ServiceError::Windows { code: 5, message: "Access is denied".into() }),
            start_panics: false,
            status: None,
        });
        let error = repository.start_service("svc").await.unwrap_err();
        assert!(
            matches!(&error, ServiceError::Windows { code: 5, message } if message == "Access is denied"),
            "expected windows error, got {error}"
        );
    }

    #[tokio::test]
    async fn maps_panic_to_internal() {
        let repository = test_bridge(StubRepository {
            start_result: Ok(()),
            start_panics: true,
            status: None,
        });
        let error = repository.start_service("svc").await.unwrap_err();
        assert!(
            matches!(&error, ServiceError::Internal { message } if message.contains("panicked")),
            "expected internal error, got {error}"
        );
    }
}
