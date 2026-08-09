use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::debug;

use crate::domain::error::ServiceError;
use crate::domain::service::{ServiceStartType, ServiceState};
use crate::queue::bridge::AsyncServiceRepository;

/// Maximum time a restart waits for a service to reach `Stopped` after issuing the stop request.
const STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
/// Interval at which restart polls the service's status while waiting for it to stop.
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Orchestrates service actions with per-service sequencing.
///
/// Each service gets its own async mutex ("lane"): actions targeting the same
/// service always run sequentially, while actions on different services run in
/// parallel — matching the parallelism rules from the spec.
///
/// Only services that receive an action ever get a lane; the map holds at most
/// one entry per acted-on service, so it cannot grow unboundedly.
pub struct ActionService {
    repository: Arc<AsyncServiceRepository>,
    lanes: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    stop_timeout: Duration,
    stop_poll_interval: Duration,
}

impl ActionService {
    pub fn new(repository: Arc<AsyncServiceRepository>) -> Self {
        Self::with_durations(repository, STOP_WAIT_TIMEOUT, STOP_POLL_INTERVAL)
    }

    fn with_durations(
        repository: Arc<AsyncServiceRepository>,
        stop_timeout: Duration,
        stop_poll_interval: Duration,
    ) -> Self {
        Self {
            repository,
            lanes: Mutex::new(HashMap::new()),
            stop_timeout,
            stop_poll_interval,
        }
    }

    pub async fn start(&self, name: &str) -> Result<(), ServiceError> {
        let _lane = self.lane(name).await;
        self.start_unlocked(name).await
    }

    pub async fn stop(&self, name: &str) -> Result<(), ServiceError> {
        let _lane = self.lane(name).await;
        self.stop_unlocked(name).await
    }

    /// Restarts a service: stops it, waits for it to actually stop, then starts it again.
    /// If the service is already stopped, this is a plain start.
    pub async fn restart(&self, name: &str) -> Result<(), ServiceError> {
        let _lane = self.lane(name).await;
        if self.query_state(name).await? != ServiceState::Stopped {
            self.stop_unlocked(name).await?;
            self.wait_until_stopped(name).await?;
        }
        self.start_unlocked(name).await
    }

    /// Starts a disabled service: sets its startup type to Manual, then starts it.
    /// The startup type is left as Manual afterwards, matching the spec.
    pub async fn force_start(&self, name: &str) -> Result<(), ServiceError> {
        let _lane = self.lane(name).await;
        self.set_start_type_unlocked(name, ServiceStartType::Manual).await?;
        self.start_unlocked(name).await
    }

    /// Changes a service's startup type; takes effect the next time it starts.
    pub async fn set_start_type(
        &self,
        name: &str,
        start_type: ServiceStartType,
    ) -> Result<(), ServiceError> {
        let _lane = self.lane(name).await;
        self.set_start_type_unlocked(name, start_type).await
    }

    /// Acquires this service's lane, creating it on first use.
    ///
    /// Returns an owned guard so the lane's `Arc` stays alive for as long as
    /// the guard is held; the guard itself is independent of `&self`.
    async fn lane(&self, name: &str) -> OwnedMutexGuard<()> {
        let lane = {
            let mut lanes = self.lanes.lock().await;
            lanes
                .entry(name.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        lane.lock_owned().await
    }

    async fn start_unlocked(&self, name: &str) -> Result<(), ServiceError> {
        self.repository.start_service(name).await
    }

    async fn stop_unlocked(&self, name: &str) -> Result<(), ServiceError> {
        self.repository.stop_service(name).await
    }

    async fn set_start_type_unlocked(
        &self,
        name: &str,
        start_type: ServiceStartType,
    ) -> Result<(), ServiceError> {
        self.repository.set_start_type(name, start_type).await
    }

    async fn query_state(&self, name: &str) -> Result<ServiceState, ServiceError> {
        let status = self.repository.query_service_status(name).await?;
        status
            .map(|status| status.state)
            .ok_or_else(|| ServiceError::service_not_found(name))
    }

    async fn wait_until_stopped(&self, name: &str) -> Result<(), ServiceError> {
        let deadline = tokio::time::Instant::now() + self.stop_timeout;
        loop {
            if self.query_state(name).await? == ServiceState::Stopped {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                debug!(service = name, timeout = ?self.stop_timeout, "stop wait timed out");
                return Err(ServiceError::Timeout { service: name.to_owned() });
            }
            tokio::time::sleep(self.stop_poll_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::domain::repository::{DynServiceRepository, ServiceRepository};
    use crate::domain::service::{ServiceInfo, ServiceRuntimeStatus};

    /// Test double recording every repository call it receives.
    struct MockRepository {
        inner: StdMutex<MockInner>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MockInner {
        state: ServiceState,
        /// When true, `stop_service` moves the service to `StopPending` but never to `Stopped`.
        stop_never_completes: bool,
        calls: Vec<String>,
    }

    impl MockRepository {
        fn new(state: ServiceState) -> Self {
            Self {
                inner: StdMutex::new(MockInner {
                    state,
                    stop_never_completes: false,
                    calls: Vec::new(),
                }),
            }
        }

        fn stop_never_completes(&self) {
            self.inner.lock().unwrap().stop_never_completes = true;
        }

        fn calls(&self) -> Vec<String> {
            self.inner.lock().unwrap().calls.clone()
        }
    }

    impl ServiceRepository for MockRepository {
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
            let inner = self.inner.lock().unwrap();
            Ok(Some(ServiceRuntimeStatus {
                name: name.to_owned(),
                state: inner.state,
                pid: None,
            }))
        }

        fn query_config(
            &self,
            _name: &str,
        ) -> Result<Option<crate::domain::service::ServiceConfig>, ServiceError> {
            Ok(None)
        }

        fn start_service(&self, name: &str) -> Result<(), ServiceError> {
            let mut inner = self.inner.lock().unwrap();
            inner.calls.push(format!("start:{name}"));
            inner.state = ServiceState::Running;
            Ok(())
        }

        fn stop_service(&self, name: &str) -> Result<(), ServiceError> {
            let mut inner = self.inner.lock().unwrap();
            inner.calls.push(format!("stop:{name}"));
            if !inner.stop_never_completes {
                inner.state = ServiceState::Stopped;
            }
            Ok(())
        }

        fn set_start_type(
            &self,
            name: &str,
            start_type: ServiceStartType,
        ) -> Result<(), ServiceError> {
            let mut inner = self.inner.lock().unwrap();
            inner.calls.push(format!("set:{start_type:?}:{name}"));
            Ok(())
        }
    }

    fn test_actions(repository: Arc<AsyncServiceRepository>) -> ActionService {
        ActionService::with_durations(
            repository,
            Duration::from_millis(100),
            Duration::from_millis(5),
        )
    }

    fn test_bridge(repository: &Arc<MockRepository>) -> Arc<AsyncServiceRepository> {
        Arc::new(AsyncServiceRepository::new(
            Arc::clone(repository) as DynServiceRepository
        ))
    }

    #[tokio::test]
    async fn start_issues_start_request() {
        let repository = Arc::new(MockRepository::new(ServiceState::Stopped));
        let actions = test_actions(test_bridge(&repository));
        actions.start("svc").await.unwrap();
        assert_eq!(repository.calls(), vec!["start:svc"]);
    }

    #[tokio::test]
    async fn restart_stops_waits_then_starts() {
        let repository = Arc::new(MockRepository::new(ServiceState::Running));
        let actions = test_actions(test_bridge(&repository));
        actions.restart("svc").await.unwrap();
        assert_eq!(repository.calls(), vec!["stop:svc", "start:svc"]);
    }

    #[tokio::test]
    async fn restart_skips_stop_when_already_stopped() {
        let repository = Arc::new(MockRepository::new(ServiceState::Stopped));
        let actions = test_actions(test_bridge(&repository));
        actions.restart("svc").await.unwrap();
        assert_eq!(repository.calls(), vec!["start:svc"]);
    }

    #[tokio::test]
    async fn restart_times_out_while_waiting_for_stop() {
        let repository = Arc::new(MockRepository::new(ServiceState::Running));
        repository.stop_never_completes();
        let actions = test_actions(test_bridge(&repository));
        let error = actions.restart("svc").await.unwrap_err();
        assert!(
            matches!(&error, ServiceError::Timeout { service } if service == "svc"),
            "expected timeout, got {error}"
        );
    }

    #[tokio::test]
    async fn force_start_sets_manual_before_starting() {
        let repository = Arc::new(MockRepository::new(ServiceState::Stopped));
        let actions = test_actions(test_bridge(&repository));
        actions.force_start("svc").await.unwrap();
        assert_eq!(repository.calls(), vec!["set:Manual:svc", "start:svc"]);
    }

    #[tokio::test]
    async fn set_start_type_issues_config_change() {
        let repository = Arc::new(MockRepository::new(ServiceState::Running));
        let actions = test_actions(test_bridge(&repository));
        actions.set_start_type("svc", ServiceStartType::Disabled).await.unwrap();
        assert_eq!(repository.calls(), vec!["set:Disabled:svc"]);
    }

    #[tokio::test]
    async fn concurrent_actions_on_same_service_are_sequential() {
        let repository = Arc::new(MockRepository::new(ServiceState::Running));
        let actions = Arc::new(test_actions(test_bridge(&repository)));
        let first = Arc::clone(&actions);
        let second = Arc::clone(&actions);
        let a = tokio::spawn(async move { first.restart("svc").await });
        let b = tokio::spawn(async move { second.restart("svc").await });
        a.await.unwrap().unwrap();
        b.await.unwrap().unwrap();
        assert_eq!(
            repository.calls(),
            vec!["stop:svc", "start:svc", "stop:svc", "start:svc"]
        );
    }
}
