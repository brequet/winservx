use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::contract::events::QueueTaskUpdated;
use crate::domain::error::ServiceError;
use crate::domain::queue::{QueueAction, QueueTask, QueueTaskStatus};

/// Port for delivering task state changes to the frontend.
/// Implemented by a Tauri adapter; the queue layer never touches Tauri itself.
pub trait TaskEventSink: Send + Sync + 'static {
    fn emit(&self, event: QueueTaskUpdated);
}

struct RegistryInner {
    next_id: u32,
    tasks: HashMap<u32, QueueTask>,
}

/// Backend-owned source of truth for the action queue.
///
/// Assigns monotonic task ids and drives the lifecycle state machine
/// (`Queued` → `Running` → `Success`/`Failed`), announcing every transition
/// through the `TaskEventSink`.
///
/// Retention follows the product rules: successful tasks are announced and
/// dropped immediately (the drawer's auto-clear is a frontend concern), while
/// failed tasks persist until the user dismisses them via `dismiss`.
pub struct TaskRegistry {
    inner: Mutex<RegistryInner>,
    sink: Arc<dyn TaskEventSink>,
}

impl TaskRegistry {
    pub fn new(sink: Arc<dyn TaskEventSink>) -> Self {
        Self {
            inner: Mutex::new(RegistryInner { next_id: 1, tasks: HashMap::new() }),
            sink,
        }
    }

    /// Registers a new task as `Queued`, announces it and returns its id.
    /// The caller drives it through `mark_running` and `complete`.
    pub async fn enqueue(&self, service_name: String, action: QueueAction) -> u32 {
        let id = {
            let mut inner = self.inner.lock().await;
            let id = inner.next_id;
            inner.next_id += 1;
            inner.tasks.insert(
                id,
                QueueTask {
                    id,
                    service_name,
                    action,
                    status: QueueTaskStatus::Queued,
                    error: None,
                },
            );
            id
        };
        self.emit_updated(id).await;
        id
    }

    /// Marks a task as `Running` (the lane has been acquired and execution
    /// started) and announces the transition.
    pub async fn mark_running(&self, id: u32) {
        self.update(id, |task| task.status = QueueTaskStatus::Running).await;
    }

    /// Marks a task's outcome. Successes are announced then dropped; failures
    /// are announced and retained until dismissed.
    pub async fn complete(&self, id: u32, result: Result<(), ServiceError>) {
        match result {
            Ok(()) => {
                self.update(id, |task| task.status = QueueTaskStatus::Success).await;
                self.inner.lock().await.tasks.remove(&id);
            }
            Err(error) => {
                self.update(id, |task| {
                    task.status = QueueTaskStatus::Failed;
                    task.error = Some(error);
                })
                .await;
            }
        }
    }

    /// Removes a task the user dismissed. Rejects tasks that are still in
    /// flight; returns whether anything was removed.
    pub async fn dismiss(&self, id: u32) -> bool {
        let mut inner = self.inner.lock().await;
        match inner.tasks.get(&id) {
            Some(task) if matches!(
                task.status,
                QueueTaskStatus::Queued | QueueTaskStatus::Running
            ) => false,
            Some(_) => {
                inner.tasks.remove(&id);
                true
            }
            None => false,
        }
    }

    /// Snapshot of the live queue: queued, running and retained failures,
    /// ordered by id. Successes are never present.
    pub async fn snapshot(&self) -> Vec<QueueTask> {
        let mut tasks: Vec<QueueTask> = self.inner.lock().await.tasks.values().cloned().collect();
        tasks.sort_by_key(|task| task.id);
        tasks
    }

    async fn update(&self, id: u32, update: impl FnOnce(&mut QueueTask)) {
        let exists = {
            let mut inner = self.inner.lock().await;
            match inner.tasks.get_mut(&id) {
                Some(task) => {
                    update(task);
                    true
                }
                None => false,
            }
        };
        if exists {
            self.emit_updated(id).await;
        }
    }

    async fn emit_updated(&self, id: u32) {
        let task = self.inner.lock().await.tasks.get(&id).cloned();
        if let Some(task) = task {
            self.sink.emit(QueueTaskUpdated { task });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use super::*;
    use crate::domain::service::ServiceStartType;

    /// Records every event it receives.
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

    fn registry() -> (TaskRegistry, Arc<RecordingSink>) {
        let sink = Arc::new(RecordingSink::new());
        let registry = TaskRegistry::new(Arc::clone(&sink) as Arc<dyn TaskEventSink>);
        (registry, sink)
    }

    fn terminal_statuses(events: &[QueueTaskUpdated], id: u32) -> Vec<QueueTaskStatus> {
        events
            .iter()
            .filter(|event| event.task.id == id)
            .map(|event| event.task.status)
            .collect()
    }

    #[tokio::test]
    async fn enqueue_assigns_increasing_ids_and_announces_queued() {
        let (registry, sink) = registry();
        let first = registry.enqueue("svc".into(), QueueAction::Start).await;
        let second = registry.enqueue("other".into(), QueueAction::Restart).await;

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(
            sink.events().iter().map(|event| event.task.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        let queued = sink.events().iter().map(|event| event.task.status).collect::<Vec<_>>();
        assert_eq!(queued, vec![QueueTaskStatus::Queued, QueueTaskStatus::Queued]);
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].id, 1);
        assert_eq!(snapshot[1].id, 2);
    }

    #[tokio::test]
    async fn task_announces_its_full_lifecycle() {
        let (registry, sink) = registry();
        let id = registry.enqueue("svc".into(), QueueAction::ForceStart).await;
        registry.mark_running(id).await;
        registry.complete(id, Ok(())).await;

        assert_eq!(
            terminal_statuses(&sink.events(), id),
            vec![
                QueueTaskStatus::Queued,
                QueueTaskStatus::Running,
                QueueTaskStatus::Success
            ]
        );
        assert!(registry.snapshot().await.is_empty(), "successful tasks are dropped");
    }

    #[tokio::test]
    async fn failure_is_retained_with_structured_error() {
        let (registry, sink) = registry();
        let id = registry.enqueue("svc".into(), QueueAction::Stop).await;
        let error = ServiceError::Windows { code: 5, message: "Access is denied".into() };
        registry.complete(id, Err(error)).await;

        assert_eq!(terminal_statuses(&sink.events(), id).last(), Some(&QueueTaskStatus::Failed));
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert!(
            matches!(&snapshot[0].error, Some(ServiceError::Windows { code: 5, .. })),
            "expected access denied, got {:?}",
            snapshot[0].error
        );
    }

    #[tokio::test]
    async fn dismiss_removes_failed_task_but_not_in_flight_one() {
        let (registry, _) = registry();
        let failed = registry.enqueue("svc".into(), QueueAction::Stop).await;
        registry
            .complete(failed, Err(ServiceError::Internal { message: "boom".into() }))
            .await;
        let in_flight = registry.enqueue("other".into(), QueueAction::Start).await;
        registry.mark_running(in_flight).await;

        assert!(!registry.dismiss(in_flight).await, "in-flight tasks cannot be dismissed");
        assert!(registry.dismiss(failed).await);
        let remaining = registry.snapshot().await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, in_flight);
    }

    #[tokio::test]
    async fn dismiss_of_unknown_id_returns_false() {
        let (registry, _) = registry();
        assert!(!registry.dismiss(42).await);
    }

    #[tokio::test]
    async fn set_start_type_action_carries_its_target() {
        let (registry, _) = registry();
        let id = registry
            .enqueue("svc".into(), QueueAction::SetStartType(ServiceStartType::Disabled))
            .await;
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot[0].id, id);
        assert_eq!(
            snapshot[0].action,
            QueueAction::SetStartType(ServiceStartType::Disabled)
        );
    }
}
