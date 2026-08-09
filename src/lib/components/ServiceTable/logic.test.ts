import { describe, expect, it } from 'vitest';
import type { ServiceInfo } from '$lib/tauri/bindings';
import {
	rowActions,
	startupOptions,
	startupClass,
	statusClass,
	stripeClass,
	STATE_LABEL,
	defaultVisibility,
	HIDEABLE_COLUMNS,
	sortAfterClick,
	sortServices
} from './logic';

function service(state: ServiceInfo['state'], startType: ServiceInfo['startType']): ServiceInfo {
	return {
		name: 'svc',
		displayName: 'Svc',
		state,
		startType,
		kind: 'win32OwnProcess',
		pid: null
	};
}

function svc(name: string, overrides: Partial<ServiceInfo> = {}): ServiceInfo {
	return {
		name,
		displayName: name,
		state: 'stopped',
		startType: 'manual',
		kind: 'win32OwnProcess',
		pid: null,
		...overrides
	};
}

describe('rowActions', () => {
	it('offers stop and restart to a running service', () => {
		expect(rowActions(service('running', 'automatic')).map((a) => a.action)).toEqual([
			'stop',
			'restart'
		]);
	});

	it('offers stop and restart to a paused service', () => {
		expect(rowActions(service('paused', 'automatic')).map((a) => a.action)).toEqual([
			'stop',
			'restart'
		]);
	});

	it('offers force start to a disabled stopped service', () => {
		const actions = rowActions(service('stopped', 'disabled'));
		expect(actions.map((a) => a.action)).toEqual(['forceStart']);
		expect(actions[0].title).toBeTruthy();
	});

	it('offers plain start to an enabled stopped service', () => {
		expect(rowActions(service('stopped', 'manual')).map((a) => a.action)).toEqual(['start']);
	});

	it('offers nothing while a state transition is pending', () => {
		for (const state of ['startPending', 'stopPending', 'continuePending', 'pausePending']) {
			expect(rowActions(service(state as never, 'manual'))).toEqual([]);
		}
	});
});

describe('startupOptions', () => {
	it('limits a win32 service to automatic, manual and disabled', () => {
		expect(startupOptions('win32OwnProcess').map((o) => o.value)).toEqual([
			'automatic',
			'manual',
			'disabled'
		]);
	});

	it('offers boot and system to drivers', () => {
		for (const kind of ['kernelDriver', 'fileSystemDriver', 'recognizerDriver']) {
			expect(startupOptions(kind as never).map((o) => o.value)).toEqual([
				'boot',
				'system',
				'automatic',
				'manual',
				'disabled'
			]);
		}
	});
});

describe('presentation classes', () => {
	it('maps states to stripe and status classes', () => {
		expect(stripeClass('running')).toBe('stripe--running');
		expect(stripeClass('startPending')).toBe('stripe--pending');
		expect(stripeClass('paused')).toBe('stripe--error');
		expect(stripeClass('stopped')).toBe('stripe--stopped');

		expect(statusClass('running')).toBe('status--running');
		expect(statusClass('stopPending')).toBe('status--pending');
		expect(statusClass('paused')).toBe('status--error');
		expect(statusClass('stopped')).toBe('status--stopped');
	});

	it('maps start types to startup classes', () => {
		expect(startupClass('disabled')).toBe('startup--disabled');
		expect(startupClass('automatic')).toBe('startup--automatic');
		expect(startupClass('boot')).toBe('startup--automatic');
		expect(startupClass('manual')).toBe('startup--manual');
		expect(startupClass(null)).toBe('startup--manual');
	});

	it('labels every state', () => {
		for (const state of Object.keys(STATE_LABEL)) {
			expect(STATE_LABEL[state as never]).toBeTruthy();
		}
	});
});

describe('sortServices', () => {
	const services = [
		svc('b', { state: 'running', startType: 'automatic' }),
		svc('c', { state: 'stopped', startType: 'disabled' }),
		svc('a', { state: 'stopped', startType: 'manual' })
	];

	it('returns the input untouched when there is no sort', () => {
		expect(sortServices(services, null)).toBe(services);
	});

	it('sorts by name ascending and descending', () => {
		expect(sortServices(services, { column: 'name', direction: 'asc' }).map((s) => s.name)).toEqual(
			['a', 'b', 'c']
		);
		expect(
			sortServices(services, { column: 'name', direction: 'desc' }).map((s) => s.name)
		).toEqual(['c', 'b', 'a']);
	});

	it('compares display names case-insensitively', () => {
		const mixed = [svc('a', { displayName: 'Zeta' }), svc('b', { displayName: 'alpha' })];
		expect(
			sortServices(mixed, { column: 'displayName', direction: 'asc' }).map((s) => s.name)
		).toEqual(['b', 'a']);
	});

	it('sorts by state using a semantic rank, running first', () => {
		const byState = [
			svc('stopped', { state: 'stopped' }),
			svc('running', { state: 'running' }),
			svc('pending', { state: 'startPending' }),
			svc('paused', { state: 'paused' }),
			svc('unknown', { state: 'unknown' })
		];
		expect(sortServices(byState, { column: 'state', direction: 'asc' }).map((s) => s.name)).toEqual(
			['running', 'pending', 'paused', 'stopped', 'unknown']
		);
		expect(
			sortServices(byState, { column: 'state', direction: 'desc' }).map((s) => s.name)
		).toEqual(['unknown', 'stopped', 'paused', 'pending', 'running']);
	});

	it('sorts by start type using a semantic rank, enabled first', () => {
		const byStart = [
			svc('disabled', { startType: 'disabled' }),
			svc('manual', { startType: 'manual' }),
			svc('auto', { startType: 'automatic' }),
			svc('boot', { startType: 'boot' })
		];
		expect(
			sortServices(byStart, { column: 'startType', direction: 'asc' }).map((s) => s.name)
		).toEqual(['auto', 'boot', 'manual', 'disabled']);
	});

	it('keeps equal values in name order as a stable tiebreaker', () => {
		const tied = [svc('z', { displayName: 'same' }), svc('a', { displayName: 'same' })];
		expect(
			sortServices(tied, { column: 'displayName', direction: 'desc' }).map((s) => s.name)
		).toEqual(['a', 'z']);
	});
});

describe('sortAfterClick', () => {
	it('cycles none → asc → desc → none on the same column', () => {
		expect(sortAfterClick(null, 'name')).toEqual({ column: 'name', direction: 'asc' });
		expect(sortAfterClick({ column: 'name', direction: 'asc' }, 'name')).toEqual({
			column: 'name',
			direction: 'desc'
		});
		expect(sortAfterClick({ column: 'name', direction: 'desc' }, 'name')).toBeNull();
	});

	it('restarts at asc when switching column', () => {
		expect(sortAfterClick({ column: 'name', direction: 'desc' }, 'state')).toEqual({
			column: 'state',
			direction: 'asc'
		});
	});
});

describe('column visibility', () => {
	it('starts with every column visible', () => {
		expect(Object.values(defaultVisibility()).every(Boolean)).toBe(true);
	});

	it('only allows hiding display name and startup', () => {
		expect(HIDEABLE_COLUMNS).toEqual(['displayName', 'startType']);
	});
});
