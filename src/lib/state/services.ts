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

/** Filters the read model by name, display name or pid; a blank query returns all. */
export function filterServices(services: ServiceInfo[], query: string): ServiceInfo[] {
	const needle = query.trim().toLowerCase();
	if (needle === '') return services;
	return services.filter(
		(service) =>
			service.name.toLowerCase().includes(needle) ||
			service.displayName.toLowerCase().includes(needle) ||
			String(service.pid ?? '').includes(needle)
	);
}
