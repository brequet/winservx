import type { ServiceInfo, ServiceKind, ServiceStartType, ServiceState } from '$lib/tauri/bindings';
import type { ServiceAction } from '$lib/queue';

export const STATE_LABEL: Record<ServiceState, string> = {
	running: 'running',
	stopped: 'stopped',
	startPending: 'starting',
	stopPending: 'stopping',
	continuePending: 'continuing',
	pausePending: 'pausing',
	paused: 'paused',
	unknown: 'unknown'
};

export interface RowAction {
	action: ServiceAction;
	label: string;
	title?: string;
}

/** The actions valid for a service's current state; spec: only show what's actually possible. */
export function rowActions(service: ServiceInfo): RowAction[] {
	switch (service.state) {
		case 'running':
		case 'paused':
			return [
				{ action: 'stop', label: 'stop' },
				{ action: 'restart', label: 'restart' }
			];
		case 'stopped':
			return service.startType === 'disabled'
				? [
						{
							action: 'forceStart',
							label: 'force start',
							title: 'disabled — sets startup type to manual, then starts'
						}
					]
				: [{ action: 'start', label: 'start' }];
		default:
			return [];
	}
}

export function stripeClass(state: ServiceState): string {
	switch (state) {
		case 'running':
			return 'stripe--running';
		case 'startPending':
		case 'stopPending':
		case 'continuePending':
		case 'pausePending':
			return 'stripe--pending';
		case 'paused':
			return 'stripe--error';
		default:
			return 'stripe--stopped';
	}
}

export function statusClass(state: ServiceState): string {
	switch (state) {
		case 'running':
			return 'status--running';
		case 'startPending':
		case 'stopPending':
		case 'continuePending':
		case 'pausePending':
			return 'status--pending';
		case 'paused':
			return 'status--error';
		default:
			return 'status--stopped';
	}
}

export function startupClass(startType: ServiceStartType | null): string {
	switch (startType) {
		case 'disabled':
			return 'startup--disabled';
		case 'boot':
		case 'system':
		case 'automatic':
			return 'startup--automatic';
		default:
			return 'startup--manual';
	}
}

const DRIVER_KINDS: ServiceKind[] = ['kernelDriver', 'fileSystemDriver', 'recognizerDriver'];

/** The start types a service can be set to; boot/system only apply to drivers. */
export function startupOptions(kind: ServiceKind): { value: ServiceStartType; label: string }[] {
	const values: ServiceStartType[] = DRIVER_KINDS.includes(kind)
		? ['boot', 'system', 'automatic', 'manual', 'disabled']
		: ['automatic', 'manual', 'disabled'];
	return values.map((value) => ({ value, label: value }));
}
