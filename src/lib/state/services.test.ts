import { describe, expect, it } from 'vitest';
import type {
	ServiceConfigChanged,
	ServiceInfo,
	ServiceStatusChanged,
	ServicesChanged
} from '$lib/tauri/bindings';
import {
	applyConfigChanged,
	applyOptimisticStartType,
	applyServicesChanged,
	applySnapshot,
	applyStatusChanged,
	filterServices,
	revertStartType
} from './services';

function service(name: string, overrides: Partial<ServiceInfo> = {}): ServiceInfo {
	return {
		name,
		displayName: `Display ${name.toUpperCase()}`,
		state: 'stopped',
		startType: 'manual',
		kind: 'win32OwnProcess',
		pid: null,
		...overrides
	};
}

const services = (): ServiceInfo[] => [
	service('a', { state: 'stopped' }),
	service('b', { state: 'running', pid: 42, startType: 'automatic' })
];

describe('applySnapshot', () => {
	it('replaces the read model with the backend snapshot', () => {
		const snapshot = [service('x', { state: 'running', pid: 7 })];
		expect(applySnapshot(services(), snapshot)).toBe(snapshot);
	});
});

describe('applyStatusChanged', () => {
	it('updates state and pid of the matching row', () => {
		const event: ServiceStatusChanged = { name: 'b', state: 'stopped', pid: null };
		expect(applyStatusChanged(services(), event)).toEqual([
			service('a', { state: 'stopped' }),
			service('b', { state: 'stopped', pid: null, startType: 'automatic' })
		]);
	});

	it('leaves the read model untouched for unknown service names', () => {
		const event: ServiceStatusChanged = { name: 'ghost', state: 'running', pid: 1 };
		expect(applyStatusChanged(services(), event)).toEqual(services());
	});
});

describe('applyConfigChanged', () => {
	it('updates display name and start type of the matching row', () => {
		const event: ServiceConfigChanged = {
			name: 'a',
			displayName: 'Alpha',
			startType: 'disabled'
		};
		expect(applyConfigChanged(services(), event)).toEqual([
			service('a', { displayName: 'Alpha', startType: 'disabled' }),
			service('b', { state: 'running', pid: 42, startType: 'automatic' })
		]);
	});
});

describe('applyServicesChanged', () => {
	it('removes deleted rows and adds new ones, keeping name order', () => {
		const event: ServicesChanged = {
			removed: ['b'],
			added: [service('c', { state: 'running' }), service('z')]
		};
		expect(applyServicesChanged(services(), event)).toEqual([
			service('a', { state: 'stopped' }),
			service('c', { state: 'running' }),
			service('z')
		]);
	});

	it('adds rows when nothing was removed', () => {
		const event: ServicesChanged = { removed: [], added: [service('c')] };
		expect(applyServicesChanged(services(), event).map((s) => s.name)).toEqual(['a', 'b', 'c']);
	});

	it('removes rows when nothing was added', () => {
		const event: ServicesChanged = { removed: ['a'], added: [] };
		expect(applyServicesChanged(services(), event).map((s) => s.name)).toEqual(['b']);
	});
});

describe('optimistic start type', () => {
	it('applies the value and remembers the previous one', () => {
		const result = applyOptimisticStartType(services(), 'b', 'disabled');
		expect(result.previous).toBe('automatic');
		expect(result.next.find((s) => s.name === 'b')?.startType).toBe('disabled');
	});

	it('remembers null when the service had no start type', () => {
		const result = applyOptimisticStartType([service('a', { startType: null })], 'a', 'manual');
		expect(result.previous).toBeNull();
	});

	it('reverts to the previous value after a failure', () => {
		const optimistic = applyOptimisticStartType(services(), 'b', 'disabled');
		expect(revertStartType(optimistic.next, 'b', 'disabled', optimistic.previous)).toEqual(
			services()
		);
	});

	it('does not revert when an event updated the value meanwhile', () => {
		const optimistic = applyOptimisticStartType(services(), 'b', 'disabled');
		const event: ServiceConfigChanged = { name: 'b', displayName: 'B', startType: 'manual' };
		const updated = applyConfigChanged(optimistic.next, event);
		expect(
			revertStartType(updated, 'b', 'disabled', 'automatic').find((s) => s.name === 'b')?.startType
		).toBe('manual');
	});
});

describe('filterServices', () => {
	const rows = services();

	it('returns everything for a blank query', () => {
		expect(filterServices(rows, '')).toBe(rows);
		expect(filterServices(rows, '   ')).toBe(rows);
	});

	it('matches the service name case-insensitively', () => {
		expect(filterServices(rows, 'B').map((s) => s.name)).toEqual(['b']);
	});

	it('matches the display name', () => {
		expect(filterServices(rows, 'display').map((s) => s.name)).toEqual(['a', 'b']);
	});

	it('matches the pid', () => {
		expect(filterServices(rows, '42').map((s) => s.name)).toEqual(['b']);
	});

	it('matches nothing for an unknown needle', () => {
		expect(filterServices(rows, 'zzz')).toEqual([]);
	});
});
