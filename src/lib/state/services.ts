import { fuzzyScore } from '$lib/search/fuzzy';
import type {
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

const NAME_BONUS = 1000;
const DISPLAY_BONUS = 800;
const PID_BONUS = 900;
const PID_EXACT_BONUS = 1100;

/**
 * Filters the read model by name, display name or pid, ranked by match
 * quality; a blank query returns all rows in their original order.
 * Equal scores keep the original (alphabetical) order.
 */
export function filterServices(services: ServiceInfo[], query: string): ServiceInfo[] {
	const needle = query.trim();
	if (needle === '') return services;
	const numeric = /^\d+$/.test(needle);

	return services
		.map((service) => ({ service, score: scoreService(service, needle, numeric) }))
		.filter((entry): entry is { service: ServiceInfo; score: number } => entry.score !== null)
		.sort((a, b) => b.score - a.score)
		.map((entry) => entry.service);
}

function scoreService(service: ServiceInfo, query: string, numericQuery: boolean): number | null {
	const scores: number[] = [];
	const name = fuzzyScore(query, service.name);
	if (name !== null) scores.push(NAME_BONUS + name);
	const display = fuzzyScore(query, service.displayName);
	if (display !== null) scores.push(DISPLAY_BONUS + display);
	if (service.pid !== null) {
		const pid = String(service.pid);
		if (pid.includes(query)) {
			const exact = numericQuery && service.pid === Number(query);
			scores.push(exact ? PID_EXACT_BONUS : PID_BONUS);
		}
	}
	return scores.length === 0 ? null : Math.max(...scores);
}
