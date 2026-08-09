import { describe, expect, it } from 'vitest';
import type { ServiceInfo } from '$lib/tauri/bindings';
import {
	rowActions,
	startupOptions,
	startupClass,
	statusClass,
	stripeClass,
	STATE_LABEL
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
