import { fuzzyScore } from '$lib/search/fuzzy';
import type {
	QueueAction,
	QueueTaskStatus,
	ServiceConfigChanged,
	ServiceInfo,
	ServiceStartType,
	ServiceStatusChanged,
	ServicesChanged
} from '$lib/tauri/bindings';

/**
 * Frontend mirror of the backend read model. Pure functions over plain data:
 * given the current array and an event, return the next array. The backend
 * remains the source of truth; these reducers only apply its contract.
 *
 * Invariant: the array stays sorted by service name, matching the backend
 * snapshot order (`ServiceCache::snapshot`).
 */

export function applySnapshot(_services: ServiceInfo[], snapshot: ServiceInfo[]): ServiceInfo[] {
	return snapshot;
}

export function applyStatusChanged(
	services: ServiceInfo[],
	event: ServiceStatusChanged
): ServiceInfo[] {
	return services.map((service) =>
		service.name === event.name ? { ...service, state: event.state, pid: event.pid } : service
	);
}

export function applyConfigChanged(
	services: ServiceInfo[],
	event: ServiceConfigChanged
): ServiceInfo[] {
	return services.map((service) =>
		service.name === event.name
			? { ...service, displayName: event.displayName, startType: event.startType }
			: service
	);
}

export function applyServicesChanged(
	services: ServiceInfo[],
	event: ServicesChanged
): ServiceInfo[] {
	const removed = new Set(event.removed);
	const kept = services.filter((service) => !removed.has(service.name));
	return [...kept, ...event.added].sort((left, right) => left.name.localeCompare(right.name));
}

/** Optimistically sets a service's start type; remembers the previous value. */
export function applyOptimisticStartType(
	services: ServiceInfo[],
	name: string,
	startType: ServiceStartType
): { next: ServiceInfo[]; previous: ServiceStartType | null } {
	const previous = services.find((service) => service.name === name)?.startType ?? null;
	return {
		next: services.map((service) => (service.name === name ? { ...service, startType } : service)),
		previous
	};
}

/**
 * Reverts a failed optimistic change. Only reverts when the service still holds
 * the optimistic value — a fresh event may have legitimately updated it.
 */
export function revertStartType(
	services: ServiceInfo[],
	name: string,
	expected: ServiceStartType,
	previous: ServiceStartType | null
): ServiceInfo[] {
	return services.map((service) =>
		service.name === name && service.startType === expected
			? { ...service, startType: previous }
			: service
	);
}

/** An optimistic startup-type change awaiting its queue task to settle. */
export interface OptimisticStartType {
	set: ServiceStartType;
	previous: ServiceStartType | null;
}

/** Optimistic startup-type changes, keyed by service name. */
export type OptimisticStartTypes = Map<string, OptimisticStartType>;

/** Records an optimistic startup-type change; overwrites any previous entry. */
export function recordOptimisticStartType(
	entries: OptimisticStartTypes,
	name: string,
	set: ServiceStartType,
	previous: ServiceStartType | null
): OptimisticStartTypes {
	const next = new Map(entries);
	next.set(name, { set, previous });
	return next;
}

/** Removes a service's optimistic startup-type entry, if present. */
export function discardOptimisticStartType(
	entries: OptimisticStartTypes,
	name: string
): OptimisticStartTypes {
	if (!entries.has(name)) return entries;
	const next = new Map(entries);
	next.delete(name);
	return next;
}

/**
 * Settles a service's optimistic entry when its queue task reaches a terminal
 * state. Successes clear the entry (the value is now real); failures revert to
 * the previous value unless a fresh event already moved the service on.
 *
 * The entry is keyed by service name and recorded synchronously on the
 * optimistic change, so a task that settles before the enqueue invoke resolves
 * still reverts correctly.
 */
export function settleOptimisticStartType(
	services: ServiceInfo[],
	entries: OptimisticStartTypes,
	task: { serviceName: string; action: QueueAction; status: QueueTaskStatus }
): { next: ServiceInfo[]; entries: OptimisticStartTypes } {
	if (typeof task.action === 'string') return { next: services, entries };
	const entry = entries.get(task.serviceName);
	if (!entry) return { next: services, entries };
	if (task.status === 'queued' || task.status === 'running') {
		return { next: services, entries };
	}
	const nextEntries = discardOptimisticStartType(entries, task.serviceName);
	if (task.status !== 'failed' || entry.set !== task.action.setStartType) {
		return { next: services, entries: nextEntries };
	}
	return {
		next: revertStartType(services, task.serviceName, entry.set, entry.previous),
		entries: nextEntries
	};
}

const NAME_BONUS = 1000;
const DISPLAY_BONUS = 800;
const PID_BONUS = 900;
const PID_EXACT_BONUS = 1100;

/**
 * Filters the read model by name, display name, pid or binary path, ranked by
 * match quality; a blank query returns all rows in their original order.
 * Name/display/pid hits always outrank path-only hits, so a binary fragment
 * never drowns an exact working-set match. Equal scores keep the original
 * (alphabetical) order.
 */
export function filterServices(services: ServiceInfo[], query: string): ServiceInfo[] {
	const needle = query.trim();
	if (needle === '') return services;
	const numeric = /^\d+$/.test(needle);

	return services
		.map((service) => ({ service, match: scoreService(service, needle, numeric) }))
		.filter((entry): entry is { service: ServiceInfo; match: ServiceMatch } => entry.match !== null)
		.sort(
			(a, b) => Number(b.match.primary) - Number(a.match.primary) || b.match.score - a.match.score
		)
		.map((entry) => entry.service);
}

interface ServiceMatch {
	score: number;
	/** True when the query hit name, display name or pid — always outranks a path-only hit. */
	primary: boolean;
}

function scoreService(
	service: ServiceInfo,
	query: string,
	numericQuery: boolean
): ServiceMatch | null {
	const scores: number[] = [];
	let primary = false;
	const name = fuzzyScore(query, service.name);
	if (name !== null) {
		scores.push(NAME_BONUS + name);
		primary = true;
	}
	const display = fuzzyScore(query, service.displayName);
	if (display !== null) {
		scores.push(DISPLAY_BONUS + display);
		primary = true;
	}
	if (service.pid !== null) {
		const pid = String(service.pid);
		if (pid.includes(query)) {
			const exact = numericQuery && service.pid === Number(query);
			scores.push(exact ? PID_EXACT_BONUS : PID_BONUS);
			primary = true;
		}
	}
	const path = fuzzyScore(query, service.binaryPath);
	if (path !== null) scores.push(path);
	if (scores.length === 0) return null;
	return { score: Math.max(...scores), primary };
}
