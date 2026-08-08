use crate::scm::DynServiceRepository;

/// Managed Tauri state shared across commands.
pub struct AppState {
    pub repository: DynServiceRepository,
}
