import type { ServiceStartType } from '$lib/tauri/bindings';
import type { QueueAction, QueueItem } from '../queue';

/** The bottom-drawer action queue: enqueue, settle, dismiss. Owns the ids. */
export interface QueueState {
	items: QueueItem[];
	nextId: number;
}

export interface NewQueueItem {
	serviceName: string;
	action: QueueAction;
	/** Target startup type, present when `action === 'setStartType'`. */
	startType?: ServiceStartType;
}

export interface EnqueueResult {
	state: QueueState;
	/** The id assigned to the new item; used to settle it later. */
	id: number;
}

/** How long a success item stays visible before it disappears. */
export const SUCCESS_CLEAR_MS = 2000;

export function createQueueState(): QueueState {
	return { items: [], nextId: 1 };
}

export function enqueue(state: QueueState, item: NewQueueItem): EnqueueResult {
	const id = state.nextId;
	return {
		state: {
			items: [{ id, status: 'inFlight', ...item }, ...state.items],
			nextId: id + 1
		},
		id
	};
}

export function settle(
	state: QueueState,
	id: number,
	outcome: { status: 'success' } | { status: 'failed'; error: string }
): QueueState {
	return {
		...state,
		items: state.items.map((item) => (item.id === id ? { ...item, ...outcome } : item))
	};
}

export function dismiss(state: QueueState, id: number): QueueState {
	return { ...state, items: state.items.filter((item) => item.id !== id) };
}

/** Actions currently in flight per service name — drives the row spinners. */
export function pendingActions(items: QueueItem[]): Map<string, QueueAction> {
	return new Map(
		items
			.filter((item) => item.status === 'inFlight')
			.map((item) => [item.serviceName, item.action])
	);
}

/** Product rule: success items auto-clear, failures persist until dismissed. */
export function shouldAutoDismiss(item: QueueItem): boolean {
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
