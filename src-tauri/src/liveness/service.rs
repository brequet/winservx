use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};

use crate::domain::error::ServiceError;
use crate::domain::watcher::{ServiceWatcher, WatcherSignal};
use crate::queue::bridge::AsyncServiceRepository;

use super::cache::ServiceCache;
use super::events::{LivenessEvent, ServicesChanged};

/// Cadence of the reconciliation poll. Notifications make changes visible
/// immediately; the poll is the safety net for missed notifications and for
/// changes the SCM does not report.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Timing knobs for the liveness pipeline, injectable for tests.
#[derive(Debug, Clone)]
pub struct LivenessConfig {
    pub poll_interval: Duration,
}

impl Default for LivenessConfig {
    fn default() -> Self {
        Self { poll_interval: POLL_INTERVAL }
    }
}

/// Port for delivering liveness events to the frontend. Implemented by a Tauri adapter.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: LivenessEvent);
}

/// Owns the service read model and keeps it fresh from two sources: SCM change
/// notifications (fast path) and a periodic reconciliation poll (safety net).
pub struct LivenessService {
    repository: Arc<AsyncServiceRepository>,
    watcher: Box<dyn ServiceWatcher>,
    cache: Arc<RwLock<ServiceCache>>,
    sink: Arc<dyn EventSink>,
    poll_interval: Duration,
    /// Readiness signal for `get_services`: carries the outcome of every full
    /// refresh attempt. The cache is written before the signal flips, so the
    /// snapshot is visible as soon as the receiver wakes.
    first_refresh: watch::Sender<Result<(), ServiceError>>,
}

/// Keeps the liveness background tasks alive for the app lifetime.
pub struct LivenessHandle {
    _tasks: Vec<tauri::async_runtime::JoinHandle<()>>,
}

impl LivenessService {
    pub fn new(
        repository: Arc<AsyncServiceRepository>,
        watcher: Box<dyn ServiceWatcher>,
        cache: Arc<RwLock<ServiceCache>>,
        sink: Arc<dyn EventSink>,
        first_refresh: watch::Sender<Result<(), ServiceError>>,
    ) -> Self {
        Self::with_config(repository, watcher, cache, sink, first_refresh, LivenessConfig::default())
    }

    fn with_config(
        repository: Arc<AsyncServiceRepository>,
        watcher: Box<dyn ServiceWatcher>,
        cache: Arc<RwLock<ServiceCache>>,
        sink: Arc<dyn EventSink>,
        first_refresh: watch::Sender<Result<(), ServiceError>>,
        config: LivenessConfig,
    ) -> Self {
        Self {
            repository,
            watcher,
            cache,
            sink,
            poll_interval: config.poll_interval,
            first_refresh,
        }
    }

    pub fn start(self, signals: mpsc::Receiver<WatcherSignal>) -> LivenessHandle {
        let service = Arc::new(self);
        let poll_service = Arc::clone(&service);
        let signal_service = Arc::clone(&service);
        let poll_task = tauri::async_runtime::spawn(async move {
            poll_service.poll_loop().await;
        });
        let signal_task = tauri::async_runtime::spawn(async move {
            signal_service.signal_loop(signals).await;
        });
        LivenessHandle {
            _tasks: vec![poll_task, signal_task],
        }
    }

    async fn poll_loop(self: &Arc<Self>) {
        self.refresh_all().await;
        let mut interval = tokio::time::interval(self.poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            self.reconcile_poll().await;
        }
    }

    async fn reconcile_poll(self: &Arc<Self>) {
        let states = match self.repository.list_states().await {
            Ok(states) => states,
            Err(error) => {
                warn!(error = %error, "status reconciliation failed");
                return;
            }
        };

        let reconcile = {
            let mut cache = self.cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.apply_states(&states)
        };
        if reconcile.needs_full_refresh {
            debug!("service set differs from cache; refreshing full snapshot");
            self.refresh_all().await;
            return;
        }
        for event in reconcile.changed {
            self.sink.emit(LivenessEvent::Status(event));
        }
    }

    async fn refresh_all(self: &Arc<Self>) {
        let fresh = match self.repository.list_services().await {
            Ok(fresh) => fresh,
            Err(error) => {
                warn!(error = %error, "full refresh failed");
                let _ = self.first_refresh.send(Err(error));
                return;
            }
        };

        let (change_set, initial) = {
            let mut cache = self.cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            let initial = cache.is_empty();
            let change_set = cache.apply_full_snapshot(fresh.clone());
            (change_set, initial)
        };

        // The cache is fully written above; the signal now tells `get_services`
        // the snapshot is ready. It flips on every attempt, so a retry after a
        // failure converges with the next successful poll.
        let _ = self.first_refresh.send(Ok(()));

        for service in &fresh {
            if let Err(error) = self.watcher.watch_service(&service.name) {
                debug!(service = %service.name, error = %error, "cannot watch service");
            }
        }
        for name in &change_set.removed {
            self.watcher.unwatch_service(name);
        }

        // The initial population reaches the frontend through `get_services`;
        // events are only meaningful once the table already exists.
        if initial {
            return;
        }
        for event in change_set.status_changed {
            self.sink.emit(LivenessEvent::Status(event));
        }
        for event in change_set.config_changed {
            self.sink.emit(LivenessEvent::Config(event));
        }
        if !change_set.added.is_empty() || !change_set.removed.is_empty() {
            self.sink.emit(LivenessEvent::Services(ServicesChanged {
                added: change_set.added,
                removed: change_set.removed,
            }));
        }
    }

    async fn signal_loop(self: &Arc<Self>, mut signals: mpsc::Receiver<WatcherSignal>) {
        while let Some(signal) = signals.recv().await {
            match signal {
                WatcherSignal::Status { name } => self.on_status_changed(&name).await,
                WatcherSignal::Config { name } => self.on_config_changed(&name).await,
                WatcherSignal::Database => {
                    debug!("service database changed; refreshing full snapshot");
                    self.refresh_all().await;
                }
            }
        }
    }

    async fn on_status_changed(self: &Arc<Self>, name: &str) {
        let status = match self.repository.query_service_status(name).await {
            Ok(Some(status)) => status,
            Ok(None) => return,
            Err(error) => {
                debug!(service = %name, error = %error, "status query failed; poll will retry");
                return;
            }
        };
        let event = {
            let mut cache = self.cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.apply_status(status)
        };
        if let Some(event) = event {
            self.sink.emit(LivenessEvent::Status(event));
        }
    }

    async fn on_config_changed(self: &Arc<Self>, name: &str) {
        let config = match self.repository.query_config(name).await {
            Ok(Some(config)) => config,
            Ok(None) => return,
            Err(error) => {
                debug!(service = %name, error = %error, "config query failed; poll will retry");
                return;
            }
        };
        let event = {
            let mut cache = self.cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.apply_config(name, config)
        };
        if let Some(event) = event {
            self.sink.emit(LivenessEvent::Config(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::domain::repository::ServiceRepository;
    use crate::domain::service::{
        ServiceConfig, ServiceInfo, ServiceKind, ServiceRuntimeStatus, ServiceStartType,
        ServiceState,
    };
    use crate::domain::watcher::NoopServiceWatcher;
    use crate::queue::bridge::AsyncServiceRepository;

    /// Records every event it receives.
    struct RecordingSink {
        events: StdMutex<Vec<LivenessEvent>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: StdMutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<LivenessEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventSink for RecordingSink {
        fn emit(&self, event: LivenessEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    /// Repository with scripted responses: snapshots served in order by
    /// `list_services` (the last one repeats), per-service statuses and configs,
    /// a status list for the reconciliation poll, and optional failures.
    struct ScriptedRepository {
        inner: StdMutex<ScriptedInner>,
    }

    #[derive(Default)]
    struct ScriptedInner {
        snapshots: Vec<Vec<ServiceInfo>>,
        statuses: HashMap<String, ServiceRuntimeStatus>,
        configs: HashMap<String, ServiceConfig>,
        states: Vec<ServiceRuntimeStatus>,
        list_failures_remaining: u32,
        list_calls: usize,
    }

    impl ScriptedRepository {
        fn new() -> Self {
            Self {
                inner: StdMutex::new(ScriptedInner::default()),
            }
        }

        fn snapshot(self, snapshot: Vec<ServiceInfo>) -> Self {
            self.inner.lock().unwrap().snapshots.push(snapshot);
            self
        }

        fn status(self, name: &str, status: ServiceRuntimeStatus) -> Self {
            self.inner.lock().unwrap().statuses.insert(name.to_owned(), status);
            self
        }

        fn config(self, name: &str, config: ServiceConfig) -> Self {
            self.inner.lock().unwrap().configs.insert(name.to_owned(), config);
            self
        }

        fn states(self, states: Vec<ServiceRuntimeStatus>) -> Self {
            self.inner.lock().unwrap().states = states;
            self
        }

        fn fail_list_services(self, failures: u32) -> Self {
            self.inner.lock().unwrap().list_failures_remaining = failures;
            self
        }
    }

    fn service(name: &str, state: ServiceState) -> ServiceInfo {
        ServiceInfo {
            name: name.to_owned(),
            display_name: name.to_uppercase(),
            state,
            start_type: Some(ServiceStartType::Automatic),
            kind: ServiceKind::Win32OwnProcess,
            pid: None,
        }
    }

    fn status(name: &str, state: ServiceState, pid: Option<u32>) -> ServiceRuntimeStatus {
        ServiceRuntimeStatus {
            name: name.to_owned(),
            state,
            pid,
        }
    }

    impl ServiceRepository for ScriptedRepository {
        fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
            let mut inner = self.inner.lock().unwrap();
            if inner.list_failures_remaining > 0 {
                inner.list_failures_remaining -= 1;
                return Err(ServiceError::Windows { code: 5, message: "Access is denied".into() });
            }
            if inner.snapshots.is_empty() {
                return Ok(Vec::new());
            }
            let index = inner.list_calls.min(inner.snapshots.len() - 1);
            inner.list_calls += 1;
            Ok(inner.snapshots[index].clone())
        }

        fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError> {
            Ok(self.inner.lock().unwrap().states.clone())
        }

        fn query_service_status(
            &self,
            name: &str,
        ) -> Result<Option<ServiceRuntimeStatus>, ServiceError> {
            Ok(self.inner.lock().unwrap().statuses.get(name).cloned())
        }

        fn query_config(&self, name: &str) -> Result<Option<ServiceConfig>, ServiceError> {
            Ok(self.inner.lock().unwrap().configs.get(name).cloned())
        }

        fn start_service(&self, _name: &str) -> Result<(), ServiceError> {
            Ok(())
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

    struct Harness {
        liveness: Arc<LivenessService>,
        first_refresh: watch::Receiver<Result<(), ServiceError>>,
        cache: Arc<RwLock<ServiceCache>>,
        sink: Arc<RecordingSink>,
        signals: mpsc::Sender<WatcherSignal>,
        signal_rx: mpsc::Receiver<WatcherSignal>,
    }

    fn harness(
        repository: Arc<AsyncServiceRepository>,
        config: LivenessConfig,
    ) -> Harness {
        let cache = Arc::new(RwLock::new(ServiceCache::default()));
        let sink = Arc::new(RecordingSink::new());
        let (first_refresh_tx, first_refresh_rx) = watch::channel(Err(ServiceError::Internal {
            message: "initial refresh pending".into(),
        }));
        let (signal_tx, signal_rx) = mpsc::channel(16);
        let liveness = Arc::new(LivenessService::with_config(
            repository,
            Box::new(NoopServiceWatcher),
            Arc::clone(&cache),
            Arc::clone(&sink) as Arc<dyn EventSink>,
            first_refresh_tx,
            config,
        ));
        Harness {
            liveness,
            first_refresh: first_refresh_rx,
            cache,
            sink,
            signals: signal_tx,
            signal_rx,
        }
    }

    fn test_repository(repository: ScriptedRepository) -> Arc<AsyncServiceRepository> {
        Arc::new(AsyncServiceRepository::new(Arc::new(repository)))
    }

    async fn wait_for_events(sink: &RecordingSink, count: usize) {
        if tokio::time::timeout(Duration::from_secs(2), async {
            while sink.events().len() < count {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_err()
        {
            panic!("timed out waiting for {count} events, got {:?}", sink.events());
        }
    }

    async fn wait_for_ready(first_refresh: &watch::Receiver<Result<(), ServiceError>>) {
        if tokio::time::timeout(Duration::from_secs(2), async {
            while first_refresh.borrow().is_err() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_err()
        {
            panic!("timed out waiting for ready signal, got {:?}", first_refresh.borrow());
        }
    }

    #[tokio::test]
    async fn first_refresh_flips_ready_after_success() {
        let harness = harness(
            test_repository(ScriptedRepository::new().snapshot(vec![service("svc", ServiceState::Running)])),
            LivenessConfig::default(),
        );

        harness.liveness.refresh_all().await;

        assert!(harness.first_refresh.has_changed().unwrap_or(false));
        assert!(harness.first_refresh.borrow().is_ok());
        let snapshot = harness.cache.read().unwrap().snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "svc");
    }

    #[tokio::test]
    async fn first_refresh_carries_error_when_refresh_fails() {
        let harness = harness(
            test_repository(ScriptedRepository::new().fail_list_services(1)),
            LivenessConfig::default(),
        );

        harness.liveness.refresh_all().await;

        assert!(harness.first_refresh.has_changed().unwrap_or(false));
        assert!(
            matches!(&*harness.first_refresh.borrow(), Err(ServiceError::Windows { code: 5, .. })),
            "expected access denied, got {:?}",
            harness.first_refresh.borrow()
        );
    }

    #[tokio::test]
    async fn later_refresh_flips_ready_after_failure() {
        let harness = harness(
            test_repository(
                ScriptedRepository::new()
                    .fail_list_services(1)
                    .snapshot(vec![service("svc", ServiceState::Running)]),
            ),
            LivenessConfig::default(),
        );

        harness.liveness.refresh_all().await;
        assert!(harness.first_refresh.borrow().is_err());

        harness.liveness.refresh_all().await;
        assert!(harness.first_refresh.borrow().is_ok());
    }

    #[tokio::test]
    async fn initial_population_emits_no_events() {
        let harness = harness(
            test_repository(ScriptedRepository::new().snapshot(vec![service("svc", ServiceState::Running)])),
            LivenessConfig::default(),
        );

        harness.liveness.refresh_all().await;

        assert!(harness.first_refresh.borrow().is_ok());
        assert!(harness.sink.events().is_empty(), "expected no events, got {:?}", harness.sink.events());
    }

    #[tokio::test]
    async fn status_signal_queries_and_emits_status_event() {
        let harness = harness(
            test_repository(ScriptedRepository::new().status(
                "svc",
                status("svc", ServiceState::Stopped, Some(5)),
            )),
            LivenessConfig::default(),
        );
        let Harness { liveness, cache, sink, signals, signal_rx, .. } = harness;
        cache.write().unwrap().apply_full_snapshot(vec![service("svc", ServiceState::Running)]);

        tokio::spawn(async move { liveness.signal_loop(signal_rx).await });
        signals.send(WatcherSignal::Status { name: "svc".into() }).await.unwrap();

        wait_for_events(&sink, 1).await;
        let events = sink.events();
        assert!(
            matches!(&events[0], LivenessEvent::Status(event)
                if event.name == "svc" && event.state == ServiceState::Stopped && event.pid == Some(5)),
            "expected status event, got {:?}",
            events[0]
        );
        let cached = cache.read().unwrap().snapshot();
        assert_eq!(cached[0].state, ServiceState::Stopped);
        assert_eq!(cached[0].pid, Some(5));
    }

    #[tokio::test]
    async fn config_signal_queries_and_emits_config_event() {
        let mut svc = service("svc", ServiceState::Running);
        svc.start_type = Some(ServiceStartType::Manual);
        let harness = harness(
            test_repository(ScriptedRepository::new().config(
                "svc",
                ServiceConfig {
                    display_name: "Svc Display".into(),
                    start_type: ServiceStartType::Automatic,
                },
            )),
            LivenessConfig::default(),
        );
        let Harness { liveness, cache, sink, signals, signal_rx, .. } = harness;
        cache.write().unwrap().apply_full_snapshot(vec![svc]);

        tokio::spawn(async move { liveness.signal_loop(signal_rx).await });
        signals.send(WatcherSignal::Config { name: "svc".into() }).await.unwrap();

        wait_for_events(&sink, 1).await;
        let events = sink.events();
        assert!(
            matches!(&events[0], LivenessEvent::Config(event)
                if event.name == "svc"
                    && event.display_name == "Svc Display"
                    && event.start_type == ServiceStartType::Automatic),
            "expected config event, got {:?}",
            events[0]
        );
    }

    #[tokio::test]
    async fn database_signal_triggers_full_refresh() {
        let harness = harness(
            test_repository(
                ScriptedRepository::new()
                    .snapshot(vec![service("a", ServiceState::Running)])
                    .snapshot(vec![
                        service("a", ServiceState::Running),
                        service("b", ServiceState::Stopped),
                    ]),
            ),
            LivenessConfig::default(),
        );
        let Harness { liveness, cache, sink, signals, signal_rx, .. } = harness;

        // Startup refresh populates the cache; the signal arrives afterwards.
        liveness.refresh_all().await;
        assert!(sink.events().is_empty());

        tokio::spawn(async move { liveness.signal_loop(signal_rx).await });
        signals.send(WatcherSignal::Database).await.unwrap();

        wait_for_events(&sink, 1).await;
        let events = sink.events();
        assert!(
            matches!(&events[0], LivenessEvent::Services(changed)
                if changed.added.len() == 1 && changed.added[0].name == "b" && changed.removed.is_empty()),
            "expected services changed event, got {:?}",
            events[0]
        );
        assert_eq!(cache.read().unwrap().snapshot().len(), 2);
    }

    #[tokio::test]
    async fn set_mismatch_triggers_full_refresh_recursion() {
        let harness = harness(
            test_repository(
                ScriptedRepository::new()
                    .states(vec![status("b", ServiceState::Running, None)])
                    .snapshot(vec![
                        service("a", ServiceState::Running),
                        service("b", ServiceState::Running),
                    ]),
            ),
            LivenessConfig::default(),
        );
        harness
            .cache
            .write()
            .unwrap()
            .apply_full_snapshot(vec![service("a", ServiceState::Running)]);

        harness.liveness.reconcile_poll().await;

        wait_for_events(&harness.sink, 1).await;
        let events = harness.sink.events();
        assert!(
            matches!(&events[0], LivenessEvent::Services(changed)
                if changed.added.len() == 1 && changed.added[0].name == "b"),
            "expected services changed event, got {:?}",
            events[0]
        );
        let names: Vec<String> =
            harness.cache.read().unwrap().snapshot().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn poll_loop_retries_after_initial_failure() {
        let harness = harness(
            test_repository(
                ScriptedRepository::new()
                    .fail_list_services(1)
                    .states(vec![status("svc", ServiceState::Running, None)])
                    .snapshot(vec![service("svc", ServiceState::Running)]),
            ),
            LivenessConfig { poll_interval: Duration::from_millis(20) },
        );

        tokio::spawn({
            let liveness = Arc::clone(&harness.liveness);
            async move { liveness.poll_loop().await }
        });

        wait_for_ready(&harness.first_refresh).await;
        let snapshot = harness.cache.read().unwrap().snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "svc");
    }
}
