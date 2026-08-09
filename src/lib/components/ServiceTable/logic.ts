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

export const KIND_LABEL: Record<ServiceKind, string> = {
	win32OwnProcess: 'own process',
	win32ShareProcess: 'shared process',
	kernelDriver: 'kernel driver',
	fileSystemDriver: 'fs driver',
	recognizerDriver: 'recognizer',
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

export type SortColumn = 'state' | 'displayName' | 'name' | 'startType';
export type SortDirection = 'asc' | 'desc';
export type SortState = { column: SortColumn; direction: SortDirection } | null;

export const SORTABLE_COLUMNS: SortColumn[] = ['state', 'displayName', 'name', 'startType'];

export type ColumnId =
	'stripe' | 'status' | 'displayName' | 'kind' | 'name' | 'startType' | 'pid' | 'actions';
export type ColumnVisibility = Record<ColumnId, boolean>;

/** Columns the user may hide from the view; the rest stay fixed. */
export const HIDEABLE_COLUMNS: ColumnId[] = ['displayName', 'startType', 'kind', 'pid'];

export function defaultVisibility(): ColumnVisibility {
	return {
		stripe: true,
		status: true,
		displayName: true,
		kind: false,
		name: true,
		startType: true,
		pid: false,
		actions: true
	};
}

/** Semantic order for the status column: what's running first, unknown last. */
const STATE_RANK: Record<ServiceState, number> = {
	running: 0,
	startPending: 1,
	stopPending: 1,
	continuePending: 1,
	pausePending: 1,
	paused: 2,
	stopped: 3,
	unknown: 4
};

/** Semantic order for the startup column: enabled first, disabled last. */
const STARTUP_RANK: Record<ServiceStartType, number> = {
	boot: 0,
	system: 0,
	automatic: 0,
	manual: 1,
	disabled: 2,
	unknown: 3
};

/** Clicking a column cycles none → asc → desc → none; switching columns starts at asc. */
export function sortAfterClick(sort: SortState, column: SortColumn): SortState {
	if (!sort || sort.column !== column) return { column, direction: 'asc' };
	return sort.direction === 'asc' ? { column, direction: 'desc' } : null;
}

function columnValue(service: ServiceInfo, column: SortColumn): string {
	switch (column) {
		case 'state':
			return service.state;
		case 'displayName':
			return service.displayName;
		case 'name':
			return service.name;
		case 'startType':
			return service.startType ?? 'unknown';
	}
}

function compare(left: string, right: string, rank?: Record<string, number>): number {
	if (rank)
		return (rank[left] ?? Number.MAX_SAFE_INTEGER) - (rank[right] ?? Number.MAX_SAFE_INTEGER);
	return left.localeCompare(right, undefined, { sensitivity: 'base' });
}

/**
 * Sorts a copy of the services; ties keep the incoming (name-ordered) order so
 * live events never make the table flicker. A null sort returns the input
 * array untouched (backend order).
 */
export function sortServices(services: ServiceInfo[], sort: SortState): ServiceInfo[] {
	if (!sort) return services;
	const { column, direction } = sort;
	const rank = column === 'state' ? STATE_RANK : column === 'startType' ? STARTUP_RANK : undefined;
	const sign = direction === 'asc' ? 1 : -1;
	return [...services].sort((left, right) => {
		const cmp = compare(columnValue(left, column), columnValue(right, column), rank);
		return cmp === 0 ? left.name.localeCompare(right.name) : cmp * sign;
	});
}
