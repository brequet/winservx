use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::debug;

use crate::domain::error::ServiceError;
use crate::domain::queue::{QueueAction, QueueTask};
use crate::domain::service::{ServiceStartType, ServiceState};
use crate::queue::registry::TaskRegistry;
use crate::runtime::bridge::AsyncServiceRepository;

/// Maximum time an action waits for a service to reach its target state.
const CONVERGE_TIMEOUT: Duration = Duration::from_secs(30);
/// Interval at which actions poll the service's status while waiting for it to converge.
const CONVERGE_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Orchestrates service actions with per-service sequencing.
///
/// Each service gets its own async mutex ("lane"): actions targeting the same
/// service always run sequentially, while actions on different services run in
/// parallel — matching the parallelism rules from the spec. Tokio mutexes are
/// FIFO, so queued tasks on one service run in enqueue order.
///
/// Tasks are owned by the `TaskRegistry`, which assigns ids and reports every
/// state transition. `enqueue` returns immediately; execution runs on spawned
/// tasks, so the UI never blocks.
///
/// A task is only `Success` once its service has actually reached the target
/// state (convergence), not when SCM accepts the request: start waits for
/// `Running`, stop for `Stopped`, restart for both. A service that never
/// converges fails the task with a timeout.
pub struct ActionService {
    repository: Arc<AsyncServiceRepository>,
    lanes: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    registry: Arc<TaskRegistry>,
    converge_timeout: Duration,
    converge_poll_interval: Duration,
}

impl ActionService {
    pub fn new(repository: Arc<AsyncServiceRepository>, registry: Arc<TaskRegistry>) -> Self {
        Self::with_durations(
            repository,
            registry,
            CONVERGE_TIMEOUT,
            CONVERGE_POLL_INTERVAL,
        )
    }

    fn with_durations(
        repository: Arc<AsyncServiceRepository>,
        registry: Arc<TaskRegistry>,
        converge_timeout: Duration,
        converge_poll_interval: Duration,
    ) -> Self {
        Self {
            repository,
            lanes: Mutex::new(HashMap::new()),
            registry,
            converge_timeout,
            converge_poll_interval,
        }
    }

    /// Queues an action against a service and returns its task id.
    /// The task starts `Queued`, flips to `Running` once it acquires the lane,
    /// and settles as `Success`/`Failed`; every transition is announced through
    /// the registry's event sink.
    pub async fn enqueue(self: &Arc<Self>, service_name: String, action: QueueAction) -> u32 {
        let id = self.registry.enqueue(service_name.clone(), action).await;
        let runner = Arc::clone(self);
        tokio::spawn(async move {
            runner.run_task(id, service_name, action).await;
        });
        id
    }

    /// Live queue snapshot (queued, running and retained failures), ordered by id.
    pub async fn snapshot(&self) -> Vec<QueueTask> {
        self.registry.snapshot().await
    }

    /// Removes a dismissed task; `false` when it is unknown or still in flight.
    pub async fn dismiss(&self, id: u32) -> bool {
        self.registry.dismiss(id).await
    }

    async fn run_task(&self, id: u32, service_name: String, action: QueueAction) {
        let _lane = self.lane(&service_name).await;
        self.registry.mark_running(id).await;
        let result = self.execute(&service_name, action).await;
        self.registry.complete(id, result).await;
    }

    async fn execute(&self, name: &str, action: QueueAction) -> Result<(), ServiceError> {
        match action {
            // Requests a start, then waits until the service actually runs.
            QueueAction::Start => {
                self.start_unlocked(name).await?;
                self.wait_for_state(name, ServiceState::Running).await
            }
            // Requests a stop, then waits until the service actually stops.
            QueueAction::Stop => {
                self.stop_unlocked(name).await?;
                self.wait_for_state(name, ServiceState::Stopped).await
            }
            // Restarts a service: stops it, waits for it to actually stop,
            // then starts it again and waits until it runs. If the service is
            // already stopped, this is a plain start.
            QueueAction::Restart => {
                if self.query_state(name).await? != ServiceState::Stopped {
                    self.stop_unlocked(name).await?;
                    self.wait_for_state(name, ServiceState::Stopped).await?;
                }
                self.start_unlocked(name).await?;
                self.wait_for_state(name, ServiceState::Running).await
            }
            // Starts a disabled service: sets its startup type to Manual, then
            // starts it and waits until it runs. The startup type is left as
            // Manual afterwards, matching the spec.
            QueueAction::ForceStart => {
                self.set_start_type_unlocked(name, ServiceStartType::Manual).await?;
                self.start_unlocked(name).await?;
                self.wait_for_state(name, ServiceState::Running).await
            }
            // Changes a service's startup type; takes effect the next time it starts.
            // There is no runtime state to converge — SCM acceptance is completion.
            QueueAction::SetStartType(start_type) => {
                self.set_start_type_unlocked(name, start_type).await
            }
        }
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

    /// Polls the service's status until it reaches `target` or the timeout
    /// expires. Reports a `Timeout` failure when the service never converges.
    async fn wait_for_state(&self, name: &str, target: ServiceState) -> Result<(), ServiceError> {
        let deadline = tokio::time::Instant::now() + self.converge_timeout;
        loop {
            if self.query_state(name).await? == target {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                debug!(service = name, target = ?target, timeout = ?self.converge_timeout, "state wait timed out");
                return Err(ServiceError::Timeout {
                    service: name.to_owned(),
                    target,
                });
            }
            tokio::time::sleep(self.converge_poll_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::domain::queue::QueueTaskStatus;
    use crate::domain::repository::{DynServiceRepository, ServiceRepository};
    use crate::domain::service::{ServiceInfo, ServiceRuntimeStatus};
    use crate::queue::events::QueueTaskUpdated;
    use crate::queue::registry::TaskEventSink;

    /// Test double recording every repository call it receives.
    struct MockRepository {
        inner: StdMutex<MockInner>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct MockInner {
        state: ServiceState,
        /// When true, `stop_service` moves the service to `StopPending` but never to `Stopped`.
        stop_never_completes: bool,
        /// When true, `start_service` moves the service to `StartPending` but never to `Running`.
        start_never_completes: bool,
        calls: Vec<String>,
    }

    impl MockRepository {
        fn new(state: ServiceState) -> Self {
            Self {
                inner: StdMutex::new(MockInner {
                    state,
                    stop_never_completes: false,
                    start_never_completes: false,
                    calls: Vec::new(),
                }),
            }
        }

        fn stop_never_completes(&self) {
            self.inner.lock().unwrap().stop_never_completes = true;
        }

        fn start_never_completes(&self) {
            self.inner.lock().unwrap().start_never_completes = true;
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
            if !inner.start_never_completes {
                inner.state = ServiceState::Running;
            }
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

    /// Records every task event it receives.
    struct RecordingSink {
        events: StdMutex<Vec<QueueTaskUpdated>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: StdMutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<QueueTaskUpdated> {
            self.events.lock().unwrap().clone()
        }
    }

    impl TaskEventSink for RecordingSink {
        fn emit(&self, event: QueueTaskUpdated) {
            self.events.lock().unwrap().push(event);
        }
    }

    struct Harness {
        actions: Arc<ActionService>,
        repository: Arc<MockRepository>,
        sink: Arc<RecordingSink>,
    }

    fn harness(state: ServiceState) -> Harness {
        let repository = Arc::new(MockRepository::new(state));
        let sink = Arc::new(RecordingSink::new());
        let registry = Arc::new(TaskRegistry::new(Arc::clone(&sink) as Arc<dyn TaskEventSink>));
        let actions = Arc::new(ActionService::with_durations(
            Arc::new(AsyncServiceRepository::new(
                Arc::clone(&repository) as DynServiceRepository
            )),
            registry,
            Duration::from_millis(100),
            Duration::from_millis(5),
        ));
        Harness {
            actions,
            repository,
            sink,
        }
    }

    /// Waits until no task is queued or running; returns the retained failures.
    async fn wait_for_idle(actions: &ActionService) -> Vec<QueueTask> {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let tasks = actions.snapshot().await;
                if tasks.iter().all(|task| task.status == QueueTaskStatus::Failed) {
                    return tasks;
                }
                if tasks.is_empty() {
                    return tasks;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("timed out waiting for the queue to settle")
    }

    /// Waits until the repository has recorded at least `count` calls.
    async fn wait_for_calls(repository: &MockRepository, count: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if repository.calls().len() >= count {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("timed out waiting for repository calls");
    }

    fn statuses(sink: &RecordingSink, id: u32) -> Vec<QueueTaskStatus> {
        sink.events()
            .iter()
            .filter(|event| event.task.id == id)
            .map(|event| event.task.status)
            .collect()
    }

    #[tokio::test]
    async fn start_task_runs_and_announces_lifecycle() {
        let harness = harness(ServiceState::Stopped);
        let Harness { actions, repository, sink, .. } = harness;

        let id = actions.enqueue("svc".into(), QueueAction::Start).await;
        assert_eq!(id, 1);
        assert!(wait_for_idle(&actions).await.is_empty(), "successes are dropped");

        assert_eq!(repository.calls(), vec!["start:svc"]);
        assert_eq!(
            statuses(&sink, id),
            vec![QueueTaskStatus::Queued, QueueTaskStatus::Running, QueueTaskStatus::Success]
        );
    }

    #[tokio::test]
    async fn restart_stops_waits_then_starts() {
        let harness = harness(ServiceState::Running);
        let Harness { actions, repository, .. } = harness;

        actions.enqueue("svc".into(), QueueAction::Restart).await;
        wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["stop:svc", "start:svc"]);
    }

    #[tokio::test]
    async fn restart_skips_stop_when_already_stopped() {
        let harness = harness(ServiceState::Stopped);
        let Harness { actions, repository, .. } = harness;

        actions.enqueue("svc".into(), QueueAction::Restart).await;
        wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["start:svc"]);
    }

    #[tokio::test]
    async fn restart_times_out_and_retains_failure() {
        let harness = harness(ServiceState::Running);
        let Harness { actions, repository, .. } = harness;
        repository.stop_never_completes();

        actions.enqueue("svc".into(), QueueAction::Restart).await;
        let failures = wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["stop:svc"]);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].status, QueueTaskStatus::Failed);
        assert!(
            matches!(&failures[0].error, Some(ServiceError::Timeout { service, target }) if service == "svc" && *target == ServiceState::Stopped),
            "expected timeout, got {:?}",
            failures[0].error
        );
    }

    #[tokio::test]
    async fn stop_times_out_when_service_never_stops() {
        let harness = harness(ServiceState::Running);
        let Harness { actions, repository, .. } = harness;
        repository.stop_never_completes();

        actions.enqueue("svc".into(), QueueAction::Stop).await;
        let failures = wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["stop:svc"]);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].status, QueueTaskStatus::Failed);
        assert!(
            matches!(&failures[0].error, Some(ServiceError::Timeout { service, target }) if service == "svc" && *target == ServiceState::Stopped),
            "expected timeout, got {:?}",
            failures[0].error
        );
    }

    #[tokio::test]
    async fn start_times_out_when_service_never_runs() {
        let harness = harness(ServiceState::Stopped);
        let Harness { actions, repository, .. } = harness;
        repository.start_never_completes();

        actions.enqueue("svc".into(), QueueAction::Start).await;
        let failures = wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["start:svc"]);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].status, QueueTaskStatus::Failed);
        assert!(
            matches!(&failures[0].error, Some(ServiceError::Timeout { service, target }) if service == "svc" && *target == ServiceState::Running),
            "expected timeout, got {:?}",
            failures[0].error
        );
    }

    #[tokio::test]
    async fn force_start_sets_manual_before_starting() {
        let harness = harness(ServiceState::Stopped);
        let Harness { actions, repository, .. } = harness;

        actions.enqueue("svc".into(), QueueAction::ForceStart).await;
        wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["set:Manual:svc", "start:svc"]);
    }

    #[tokio::test]
    async fn set_start_type_issues_config_change() {
        let harness = harness(ServiceState::Running);
        let Harness { actions, repository, .. } = harness;

        actions.enqueue("svc".into(), QueueAction::SetStartType(ServiceStartType::Disabled)).await;
        wait_for_idle(&actions).await;

        assert_eq!(repository.calls(), vec!["set:Disabled:svc"]);
    }

    #[tokio::test]
    async fn concurrent_actions_on_same_service_are_sequential() {
        let harness = harness(ServiceState::Running);
        let Harness { actions, repository, .. } = harness;

        let first = actions.enqueue("svc".into(), QueueAction::Restart).await;
        assert_eq!(first, 1);
        // Wait for the first restart to fully complete before enqueueing the
        // second, so execution order is deterministic.
        wait_for_calls(&repository, 2).await;
        let second = actions.enqueue("svc".into(), QueueAction::Restart).await;
        assert_eq!(second, 2);
        wait_for_idle(&actions).await;

        assert_eq!(
            repository.calls(),
            vec!["stop:svc", "start:svc", "stop:svc", "start:svc"]
        );
    }
}
