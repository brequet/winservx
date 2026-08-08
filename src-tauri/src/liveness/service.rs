use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::domain::repository::DynServiceRepository;
use crate::domain::watcher::{ServiceWatcher, WatcherSignal};

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
    repository: DynServiceRepository,
    watcher: Box<dyn ServiceWatcher>,
    cache: Arc<RwLock<ServiceCache>>,
    sink: Arc<dyn EventSink>,
}

/// Keeps the liveness background tasks alive for the app lifetime.
pub struct LivenessHandle {
    _tasks: Vec<tauri::async_runtime::JoinHandle<()>>,
}

impl LivenessService {
    pub fn new(
        repository: DynServiceRepository,
        watcher: Box<dyn ServiceWatcher>,
        cache: Arc<RwLock<ServiceCache>>,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            repository,
            watcher,
            cache,
            sink,
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
        let repository = Arc::clone(&self.repository);
        let states = match tauri::async_runtime::spawn_blocking(move || repository.list_states())
            .await
        {
            Ok(Ok(states)) => states,
            Ok(Err(error)) => {
                warn!(error = %error, "status reconciliation failed");
                return;
            }
            Err(panic) => {
                error!(panic = %panic, "status reconciliation task panicked");
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
        let repository = Arc::clone(&self.repository);
        let fresh = match tauri::async_runtime::spawn_blocking(move || repository.list_services())
            .await
        {
            Ok(Ok(fresh)) => fresh,
            Ok(Err(error)) => {
                warn!(error = %error, "full refresh failed");
                return;
            }
            Err(panic) => {
                error!(panic = %panic, "full refresh task panicked");
                return;
            }
        };

        let (change_set, initial) = {
            let mut cache = self.cache.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            let initial = cache.is_empty();
            let change_set = cache.apply_full_snapshot(fresh.clone());
            (change_set, initial)
        };

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
        let repository = Arc::clone(&self.repository);
        let query_name = name.to_owned();
        let status = match tauri::async_runtime::spawn_blocking(move || {
            repository.query_service_status(&query_name)
        })
        .await
        {
            Ok(Ok(Some(status))) => status,
            Ok(Ok(None)) => return,
            Ok(Err(error)) => {
                debug!(service = %name, error = %error, "status query failed; poll will retry");
                return;
            }
            Err(panic) => {
                error!(panic = %panic, "status query task panicked");
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
        let repository = Arc::clone(&self.repository);
        let query_name = name.to_owned();
        let config = match tauri::async_runtime::spawn_blocking(move || {
            repository.query_config(&query_name)
        })
        .await
        {
            Ok(Ok(Some(config))) => config,
            Ok(Ok(None)) => return,
            Ok(Err(error)) => {
                debug!(service = %name, error = %error, "config query failed; poll will retry");
                return;
            }
            Err(panic) => {
                error!(panic = %panic, "config query task panicked");
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
