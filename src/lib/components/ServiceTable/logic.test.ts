import { describe, expect, it } from 'vitest';
import type { ServiceInfo } from '$lib/tauri/bindings';
import {
	rowActions,
	copyItems,
	isTransitioning,
	startupOptions,
	startupClass,
	statusClass,
	stripeClass,
	STATE_LABEL,
	logonLabel,
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
		pid: null,
		binaryPath: '',
		startName: null
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
		binaryPath: '',
		startName: null,
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

describe('copyItems', () => {
	it('offers name, path and pid when all are available', () => {
		const svc = service('running', 'automatic');
		svc.binaryPath = 'C:\\bin\\svc.exe';
		svc.pid = 4242;
		expect(copyItems(svc)).toEqual([
			{ id: 'name', label: 'Copy service name', text: 'svc' },
			{ id: 'path', label: 'Copy executable path', text: 'C:\\bin\\svc.exe' },
			{ id: 'pid', label: 'Copy PID', text: '4242' }
		]);
	});

	it('omits the path when unknown and the pid when the service has none', () => {
		expect(copyItems(service('stopped', 'manual')).map((item) => item.id)).toEqual(['name']);
	});
});

describe('isTransitioning', () => {
	it('flags pending states, not stable ones', () => {
		for (const state of ['startPending', 'stopPending', 'continuePending', 'pausePending']) {
			expect(isTransitioning(state as never)).toBe(true);
		}
		for (const state of ['running', 'stopped', 'paused', 'unknown']) {
			expect(isTransitioning(state as never)).toBe(false);
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

describe('logonLabel', () => {
	it('friendly-labels the well-known service accounts', () => {
		expect(logonLabel('LocalSystem')).toBe('Local System');
		expect(logonLabel('NT AUTHORITY\\LocalService')).toBe('Local Service');
		expect(logonLabel('.\\LocalService')).toBe('Local Service');
		expect(logonLabel('NT AUTHORITY\\NetworkService')).toBe('Network Service');
		expect(logonLabel('.\\NetworkService')).toBe('Network Service');
	});

	it('passes unknown accounts through untouched', () => {
		expect(logonLabel('DOMAIN\\alice')).toBe('DOMAIN\\alice');
		expect(logonLabel('LocalService')).toBe('Local Service');
	});

	it('renders missing accounts as a dash', () => {
		expect(logonLabel(null)).toBe('—');
		expect(logonLabel('')).toBe('—');
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

	it('sorts kind and logon account by their displayed labels', () => {
		const byDetails = [
			svc('shared', { kind: 'win32ShareProcess', startName: 'NT AUTHORITY\\NetworkService' }),
			svc('own', { kind: 'win32OwnProcess', startName: 'LocalSystem' }),
			svc('missing', { kind: 'unknown', startName: null })
		];
		expect(
			sortServices(byDetails, { column: 'kind', direction: 'asc' }).map((s) => s.name)
		).toEqual(['own', 'shared', 'missing']);
		expect(
			sortServices(byDetails, { column: 'startName', direction: 'asc' }).map((s) => s.name)
		).toEqual(['own', 'shared', 'missing']);
	});

	it('sorts PIDs numerically and keeps missing values last', () => {
		const byPid = [svc('ten', { pid: 10 }), svc('two', { pid: 2 }), svc('missing', { pid: null })];
		expect(sortServices(byPid, { column: 'pid', direction: 'asc' }).map((s) => s.name)).toEqual([
			'two',
			'ten',
			'missing'
		]);
		expect(sortServices(byPid, { column: 'pid', direction: 'desc' }).map((s) => s.name)).toEqual([
			'ten',
			'two',
			'missing'
		]);
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
	it('starts with fixed columns visible and the optional ones hidden', () => {
		const visible = defaultVisibility();
		expect(visible.stripe).toBe(true);
		expect(visible.status).toBe(true);
		expect(visible.displayName).toBe(true);
		expect(visible.name).toBe(true);
		expect(visible.startType).toBe(true);
		expect(visible.actions).toBe(true);
		expect(visible.kind).toBe(false);
		expect(visible.startName).toBe(false);
		expect(visible.pid).toBe(false);
	});

	it('only allows hiding the optional columns', () => {
		expect(HIDEABLE_COLUMNS).toEqual(['displayName', 'startType', 'kind', 'startName', 'pid']);
	});
});
