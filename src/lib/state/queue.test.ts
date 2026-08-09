import { afterEach, describe, expect, it, vi } from 'vitest';
import {
	createQueueState,
	dismiss,
	enqueue,
	pendingActions,
	scheduleSuccessDismiss,
	settle,
	shouldAutoDismiss,
	SUCCESS_CLEAR_MS
} from './queue';

afterEach(() => {
	vi.useRealTimers();
});

describe('enqueue', () => {
	it('prepends an in-flight item and assigns increasing ids', () => {
		const first = enqueue(createQueueState(), { serviceName: 'svc', action: 'start' });
		const second = enqueue(first.state, { serviceName: 'other', action: 'restart' });

		expect(first.id).toBe(1);
		expect(first.state.items).toEqual([
			{ id: 1, serviceName: 'svc', action: 'start', status: 'inFlight' }
		]);
		expect(second.id).toBe(2);
		expect(second.state.items[0].id).toBe(2);
		expect(second.state.items[1].id).toBe(1);
	});

	it('carries the start type for startup changes', () => {
		const { state } = enqueue(createQueueState(), {
			serviceName: 'svc',
			action: 'setStartType',
			startType: 'disabled'
		});
		expect(state.items[0]).toMatchObject({ action: 'setStartType', startType: 'disabled' });
	});
});

describe('settle and dismiss', () => {
	it('marks an item as failed with its error', () => {
		const { state, id } = enqueue(createQueueState(), { serviceName: 'svc', action: 'stop' });
		const settled = settle(state, id, { status: 'failed', error: 'access denied' });
		expect(settled.items[0]).toMatchObject({ status: 'failed', error: 'access denied' });
	});

	it('marks an item as successful', () => {
		const { state, id } = enqueue(createQueueState(), { serviceName: 'svc', action: 'start' });
		expect(settle(state, id, { status: 'success' }).items[0].status).toBe('success');
	});

	it('settling an unknown id leaves the queue unchanged', () => {
		const { state } = enqueue(createQueueState(), { serviceName: 'svc', action: 'start' });
		expect(settle(state, 999, { status: 'success' })).toEqual(state);
	});

	it('dismisses an item', () => {
		const { state, id } = enqueue(createQueueState(), { serviceName: 'svc', action: 'start' });
		expect(dismiss(state, id).items).toEqual([]);
	});
});

describe('pendingActions', () => {
	it('maps in-flight items per service, skipping settled ones', () => {
		const { state, id } = enqueue(createQueueState(), { serviceName: 'done', action: 'stop' });
		const settled = settle(state, id, { status: 'success' });
		const withPending = enqueue(settled, { serviceName: 'svc', action: 'restart' });
		expect(pendingActions(withPending.state.items)).toEqual(new Map([['svc', 'restart']]));
	});
});

describe('auto-dismiss rule', () => {
	it('success items clear, failures persist until dismissed', () => {
		expect(shouldAutoDismiss({ status: 'success' } as never)).toBe(true);
		expect(shouldAutoDismiss({ status: 'failed' } as never)).toBe(false);
		expect(shouldAutoDismiss({ status: 'inFlight' } as never)).toBe(false);
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
