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
        Self {
            repository,
            watcher,
            cache,
            sink,
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
        let mut interval = tokio::time::interval(POLL_INTERVAL);
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
    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::domain::repository::ServiceRepository;
    use crate::domain::service::{
        ServiceConfig, ServiceInfo, ServiceKind, ServiceRuntimeStatus, ServiceStartType,
        ServiceState,
    };
    use crate::domain::watcher::NoopServiceWatcher;
    use crate::queue::bridge::AsyncServiceRepository;

    struct TestSink;

    impl EventSink for TestSink {
        fn emit(&self, _event: LivenessEvent) {}
    }

    /// Repository whose `list_services` fails until it has been called a given number of times.
    struct FlakyRepository {
        failures: StdMutex<u32>,
    }

    impl FlakyRepository {
        fn new(failures: u32) -> Self {
            Self {
                failures: StdMutex::new(failures),
            }
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

    impl ServiceRepository for FlakyRepository {
        fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
            let mut failures = self.failures.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err(ServiceError::Windows { code: 5, message: "Access is denied".into() });
            }
            Ok(vec![service("svc", ServiceState::Running)])
        }

        fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError> {
            Ok(Vec::new())
        }

        fn query_service_status(
            &self,
            _name: &str,
        ) -> Result<Option<ServiceRuntimeStatus>, ServiceError> {
            Ok(None)
        }

        fn query_config(&self, _name: &str) -> Result<Option<ServiceConfig>, ServiceError> {
            Ok(None)
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

    fn test_liveness(
        repository: Arc<AsyncServiceRepository>,
    ) -> (Arc<LivenessService>, watch::Receiver<Result<(), ServiceError>>) {
        let cache = Arc::new(RwLock::new(ServiceCache::default()));
        let (first_refresh_tx, first_refresh_rx) = watch::channel(Err(ServiceError::Internal {
            message: "initial refresh pending".into(),
        }));
        let liveness = Arc::new(LivenessService::new(
            repository,
            Box::new(NoopServiceWatcher),
            cache,
            Arc::new(TestSink),
            first_refresh_tx,
        ));
        (liveness, first_refresh_rx)
    }

    #[tokio::test]
    async fn first_refresh_flips_ready_after_success() {
        let repository =
            Arc::new(AsyncServiceRepository::new(Arc::new(FlakyRepository::new(0))));
        let (liveness, rx) = test_liveness(repository);

        liveness.refresh_all().await;

        assert!(rx.has_changed().unwrap_or(false));
        assert!(rx.borrow().is_ok());
        let cache = liveness.cache.read().unwrap();
        let snapshot = cache.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "svc");
    }

    #[tokio::test]
    async fn first_refresh_carries_error_when_refresh_fails() {
        let repository =
            Arc::new(AsyncServiceRepository::new(Arc::new(FlakyRepository::new(1))));
        let (liveness, rx) = test_liveness(repository);

        liveness.refresh_all().await;

        assert!(rx.has_changed().unwrap_or(false));
        assert!(
            matches!(&*rx.borrow(), Err(ServiceError::Windows { code: 5, .. })),
            "expected access denied, got {:?}",
            rx.borrow()
        );
    }

    #[tokio::test]
    async fn later_refresh_flips_ready_after_failure() {
        let repository =
            Arc::new(AsyncServiceRepository::new(Arc::new(FlakyRepository::new(1))));
        let (liveness, rx) = test_liveness(repository);

        liveness.refresh_all().await;
        assert!(rx.borrow().is_err());

        liveness.refresh_all().await;
        assert!(rx.borrow().is_ok());
    }
}
