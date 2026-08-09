import type { QueueTask } from '$lib/tauri/bindings';

/** The bottom-drawer action queue, a projection of backend task events. */
export type QueueState = QueueTask[];

/** How long a success item stays visible before it disappears. */
export const SUCCESS_CLEAR_MS = 2000;

export function createQueueState(): QueueState {
	return [];
}

/** Replaces the queue with the backend snapshot (ordered by id). */
export function applyQueueSnapshot(_items: QueueTask[], snapshot: QueueTask[]): QueueTask[] {
	return snapshot;
}

/** Upserts a task by id, keeping the array ordered by id (backend order). */
export function applyTaskChanged(items: QueueTask[], task: QueueTask): QueueTask[] {
	const next = items.some((item) => item.id === task.id)
		? items.map((item) => (item.id === task.id ? task : item))
		: [...items, task];
	return next.sort((left, right) => left.id - right.id);
}

/**
 * Inserts a locally-known task after an invoke resolves, unless the backend
 * already reported that id. Backend events are authoritative: re-inserting a
 * stale `queued` item after the lifecycle completed would leave a phantom
 * entry with a stuck spinner.
 */
export function insertPendingTask(items: QueueTask[], task: QueueTask): QueueTask[] {
	if (items.some((item) => item.id === task.id)) return items;
	return applyTaskChanged(items, task);
}

export function dismiss(items: QueueTask[], id: number): QueueTask[] {
	return items.filter((item) => item.id !== id);
}

/** Tasks in flight (queued or running) per service name — drives the row spinners. */
export function pendingActions(items: QueueTask[]): Map<string, QueueTask> {
	return new Map(
		items
			.filter((item) => item.status === 'queued' || item.status === 'running')
			.map((item) => [item.serviceName, item])
	);
}

/** Product rule: success items auto-clear, failures persist until dismissed. */
export function shouldAutoDismiss(item: QueueTask): boolean {
	return item.status === 'success';
}

/** Schedules the removal of a settled success item; returns a cancel function. */
export function scheduleSuccessDismiss(
	id: number,
	dismissItem: (id: number) => void,
	delayMs: number = SUCCESS_CLEAR_MS
): () => void {
	const timer = setTimeout(() => dismissItem(id), delayMs);
	return () => clearTimeout(timer);
}
