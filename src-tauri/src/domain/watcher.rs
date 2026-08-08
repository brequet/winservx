use crate::domain::error::ServiceError;

/// Signals emitted by the SCM watcher. The consumer re-queries fresh state in response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherSignal {
    Status { name: String },
    Config { name: String },
    Database,
}

/// Port for subscribing to SCM change notifications. Implemented by platform adapters.
pub trait ServiceWatcher: Send + Sync {
    fn watch_service(&self, name: &str) -> Result<(), ServiceError>;
    fn unwatch_service(&self, name: &str);
}

/// Watcher that never delivers signals; used when SCM subscriptions are unavailable.
/// The reconciliation poll alone keeps the cache fresh.
pub struct NoopServiceWatcher;

impl ServiceWatcher for NoopServiceWatcher {
    fn watch_service(&self, _name: &str) -> Result<(), ServiceError> {
        Ok(())
    }

    fn unwatch_service(&self, _name: &str) {}
}
