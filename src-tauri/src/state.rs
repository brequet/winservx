use crate::domain::repository::DynServiceRepository;

/// Managed Tauri state shared across commands.
pub struct AppState {
    pub repository: DynServiceRepository,
}
