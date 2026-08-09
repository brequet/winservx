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

/** Friendly labels for the well-known SCM service accounts; others pass through. */
export function logonLabel(startName: string | null): string {
	if (!startName) return '—';
	switch (startName) {
		case 'LocalSystem':
			return 'Local System';
		case 'NT AUTHORITY\\LocalService':
		case '.\\LocalService':
		case 'LocalService':
			return 'Local Service';
		case 'NT AUTHORITY\\NetworkService':
		case '.\\NetworkService':
		case 'NetworkService':
			return 'Network Service';
		default:
			return startName;
	}
}

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

export type CopyItemId = 'name' | 'path' | 'pid';

export interface CopyItem {
	id: CopyItemId;
	label: string;
	text: string;
}

/** Copyable technical details for the overflow menu; only what's available. */
export function copyItems(service: ServiceInfo): CopyItem[] {
	const items: CopyItem[] = [{ id: 'name', label: 'Copy service name', text: service.name }];
	if (service.binaryPath) {
		items.push({ id: 'path', label: 'Copy executable path', text: service.binaryPath });
	}
	if (service.pid !== null) {
		items.push({ id: 'pid', label: 'Copy PID', text: String(service.pid) });
	}
	return items;
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

/** Service states that are mid-transition; no row actions are valid. */
const TRANSITIONING_STATES: ServiceState[] = [
	'startPending',
	'stopPending',
	'continuePending',
	'pausePending'
];

export function isTransitioning(state: ServiceState): boolean {
	return TRANSITIONING_STATES.includes(state);
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

export type SortColumn =
	'state' | 'displayName' | 'kind' | 'name' | 'startType' | 'startName' | 'pid';
export type SortDirection = 'asc' | 'desc';
export type SortState = { column: SortColumn; direction: SortDirection } | null;

export const SORTABLE_COLUMNS: SortColumn[] = [
	'state',
	'displayName',
	'kind',
	'name',
	'startType',
	'startName',
	'pid'
];

export type ColumnId =
	| 'stripe'
	| 'status'
	| 'displayName'
	| 'kind'
	| 'name'
	| 'startType'
	| 'startName'
	| 'pid'
	| 'actions';
export type ColumnVisibility = Record<ColumnId, boolean>;

/** Columns the user may hide from the view; the rest stay fixed. */
export const HIDEABLE_COLUMNS: ColumnId[] = [
	'displayName',
	'startType',
	'kind',
	'startName',
	'pid'
];

export function defaultVisibility(): ColumnVisibility {
	return {
		stripe: true,
		status: true,
		displayName: true,
		kind: false,
		name: true,
		startType: true,
		startName: false,
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

function columnValue(service: ServiceInfo, column: SortColumn): string | number | null {
	switch (column) {
		case 'state':
			return service.state;
		case 'displayName':
			return service.displayName;
		case 'kind':
			return KIND_LABEL[service.kind];
		case 'name':
			return service.name;
		case 'startType':
			return service.startType ?? 'unknown';
		case 'startName':
			return service.startName ? logonLabel(service.startName) : null;
		case 'pid':
			return service.pid;
	}
}

function compare(
	left: string | number,
	right: string | number,
	rank?: Record<string, number>
): number {
	if (rank)
		return (
			(rank[String(left)] ?? Number.MAX_SAFE_INTEGER) -
			(rank[String(right)] ?? Number.MAX_SAFE_INTEGER)
		);
	if (typeof left === 'number' && typeof right === 'number') return left - right;
	return String(left).localeCompare(String(right), undefined, { sensitivity: 'base' });
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
		const leftValue = columnValue(left, column);
		const rightValue = columnValue(right, column);
		if (leftValue === null || rightValue === null) {
			if (leftValue === rightValue) return left.name.localeCompare(right.name);
			return leftValue === null ? 1 : -1;
		}
		const cmp = compare(leftValue, rightValue, rank);
		return cmp === 0 ? left.name.localeCompare(right.name) : cmp * sign;
	});
}
