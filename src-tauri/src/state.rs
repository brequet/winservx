use std::sync::{Arc, RwLock};

use tracing::error;

use crate::domain::actions::ActionService;
use crate::domain::repository::DynServiceRepository;
use crate::liveness::cache::ServiceCache;
use crate::liveness::events::LivenessEvent;
use crate::liveness::service::{EventSink, LivenessHandle};

/// Managed Tauri state shared across commands.
pub struct AppState {
    pub repository: DynServiceRepository,
    pub cache: Arc<RwLock<ServiceCache>>,
    pub actions: Arc<ActionService>,
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
