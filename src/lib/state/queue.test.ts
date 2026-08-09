import { afterEach, describe, expect, it, vi } from 'vitest';
import type { QueueTask, QueueTaskStatus } from '$lib/tauri/bindings';
import {
	applyQueueSnapshot,
	applyTaskChanged,
	createQueueState,
	dismiss,
	insertPendingTask,
	pendingActions,
	scheduleSuccessDismiss,
	shouldAutoDismiss,
	SUCCESS_CLEAR_MS
} from './queue';

function task(
	id: number,
	status: QueueTaskStatus,
	serviceName = 'svc',
	action: QueueTask['action'] = 'start'
): QueueTask {
	return { id, serviceName, action, status, error: null };
}

afterEach(() => {
	vi.useRealTimers();
});

describe('applyTaskChanged', () => {
	it('appends new tasks and keeps the queue ordered by id', () => {
		const first = applyTaskChanged(createQueueState(), task(3, 'queued', 'a'));
		const withOlder = applyTaskChanged(first, task(1, 'running', 'b'));

		expect(withOlder.map((item) => item.id)).toEqual([1, 3]);
	});

	it('upserts an existing task by id', () => {
		const queued = applyTaskChanged(createQueueState(), task(7, 'queued'));
		const running = applyTaskChanged(queued, task(7, 'running'));

		expect(running).toEqual([task(7, 'running')]);
	});

	it('replaces the whole queue with the backend snapshot', () => {
		const items = applyTaskChanged(createQueueState(), task(9, 'failed'));
		expect(applyQueueSnapshot(items, [task(2, 'running')])).toEqual([task(2, 'running')]);
	});
});

describe('insertPendingTask', () => {
	it('inserts a queued item when the backend has not reported the task yet', () => {
		const items = insertPendingTask(createQueueState(), task(5, 'queued'));
		expect(items).toEqual([task(5, 'queued')]);
	});

	it('does not re-insert a stale queued item the backend already completed', () => {
		const lifecycle = applyTaskChanged(
			applyTaskChanged(applyTaskChanged(createQueueState(), task(5, 'queued')), task(5, 'running')),
			task(5, 'success')
		);
		expect(insertPendingTask(lifecycle, task(5, 'queued'))).toEqual([task(5, 'success')]);
	});
});

describe('dismiss', () => {
	it('removes an item by id', () => {
		const items = applyTaskChanged(
			applyTaskChanged(createQueueState(), task(1, 'failed')),
			task(2, 'running')
		);
		expect(dismiss(items, 1).map((item) => item.id)).toEqual([2]);
	});
});

describe('pendingActions', () => {
	it('maps queued and running tasks per service, skipping settled ones', () => {
		const items = applyTaskChanged(
			applyTaskChanged(createQueueState(), task(1, 'queued', 'first')),
			task(2, 'running', 'second')
		);
		const settled = applyTaskChanged(items, task(3, 'success', 'done'));
		expect(pendingActions(settled).size).toBe(2);
		expect(pendingActions(settled).get('first')?.status).toBe('queued');
		expect(pendingActions(settled).get('second')?.status).toBe('running');
	});
});

describe('auto-dismiss rule', () => {
	it('success items clear, failures persist until dismissed', () => {
		expect(shouldAutoDismiss(task(1, 'success'))).toBe(true);
		expect(shouldAutoDismiss(task(1, 'failed'))).toBe(false);
		expect(shouldAutoDismiss(task(1, 'queued'))).toBe(false);
		expect(shouldAutoDismiss(task(1, 'running'))).toBe(false);
	});

	it('schedules the dismissal of a success item after the clear delay', () => {
		vi.useFakeTimers();
		const dismissItem = vi.fn();
		scheduleSuccessDismiss(7, dismissItem);
		expect(dismissItem).not.toHaveBeenCalled();
		vi.advanceTimersByTime(SUCCESS_CLEAR_MS);
		expect(dismissItem).toHaveBeenCalledWith(7);
	});

	it('cancelling prevents the dismissal', () => {
		vi.useFakeTimers();
		const dismissItem = vi.fn();
		const cancel = scheduleSuccessDismiss(7, dismissItem);
		cancel();
		vi.advanceTimersByTime(SUCCESS_CLEAR_MS);
		expect(dismissItem).not.toHaveBeenCalled();
	});
});
