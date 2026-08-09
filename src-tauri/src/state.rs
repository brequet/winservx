use std::sync::{Arc, RwLock};

use tokio::sync::{watch, Mutex};
use tracing::error;

use crate::domain::error::ServiceError;
use crate::liveness::cache::ServiceCache;
use crate::liveness::events::LivenessEvent;
use crate::liveness::service::{EventSink, LivenessHandle};
use crate::queue::actions::ActionService;
use crate::queue::events::QueueTaskUpdated;
use crate::queue::registry::TaskEventSink;

/// Managed Tauri state shared across commands.
pub struct AppState {
    pub cache: Arc<RwLock<ServiceCache>>,
    pub actions: Arc<ActionService>,
    /// Carries the outcome of the latest full refresh; `get_services` awaits
    /// the first flip and then reads the cache. Mutex-wrapped because awaiting
    /// a `watch` flip needs `&mut` on the receiver.
    pub first_refresh: Mutex<watch::Receiver<Result<(), ServiceError>>>,
    pub _liveness: LivenessHandle,
}

/// Routes liveness events to the WebView through Tauri's typed event system.
pub struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: LivenessEvent) {
        let result = match &event {
            LivenessEvent::Status(e) => tauri_specta::Event::emit(e, &self.app),
            LivenessEvent::Config(e) => tauri_specta::Event::emit(e, &self.app),
            LivenessEvent::Services(e) => tauri_specta::Event::emit(e, &self.app),
        };
        if let Err(error) = result {
            error!(?event, error = %error, "failed to emit liveness event");
        }
    }
}

impl TaskEventSink for TauriEventSink {
    fn emit(&self, event: QueueTaskUpdated) {
        if let Err(error) = tauri_specta::Event::emit(&event, &self.app) {
            error!(?event, error = %error, "failed to emit queue task event");
        }
    }
}
