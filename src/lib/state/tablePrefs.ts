import {
	defaultVisibility,
	HIDEABLE_COLUMNS,
	SORTABLE_COLUMNS,
	type ColumnId,
	type ColumnVisibility,
	type SortColumn,
	type SortState
} from '$lib/components/ServiceTable/logic';

const STORAGE_KEY = 'winservx:table-prefs';

export interface TablePrefs {
	sort: SortState;
	visible: ColumnVisibility;
}

/** Reads persisted sort/visibility; falls back to defaults when absent or invalid. */
export function loadTablePrefs(): TablePrefs {
	if (typeof localStorage === 'undefined') return { sort: null, visible: defaultVisibility() };
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return { sort: null, visible: defaultVisibility() };
	try {
		const parsed: unknown = JSON.parse(raw);
		if (!parsed || typeof parsed !== 'object') {
			return { sort: null, visible: defaultVisibility() };
		}
		const { sort: rawSort, visible: rawVisible } = parsed as Record<string, unknown>;
		const visible = parseVisibility(rawVisible);
		const sort = parseSort(rawSort);
		if (sort && !visible[sort.column as ColumnId]) return { sort: null, visible };
		return { sort, visible };
	} catch {
		return { sort: null, visible: defaultVisibility() };
	}
}

export function saveTablePrefs(sort: SortState, visible: ColumnVisibility): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify({ sort, visible }));
}

function parseSort(value: unknown): SortState {
	if (!value || typeof value !== 'object') return null;
	const { column, direction } = value as Record<string, unknown>;
	if (
		typeof column === 'string' &&
		(direction === 'asc' || direction === 'desc') &&
		SORTABLE_COLUMNS.includes(column as SortColumn)
	) {
		return { column: column as SortColumn, direction };
	}
	return null;
}

function parseVisibility(value: unknown): ColumnVisibility {
	const visible = defaultVisibility();
	if (!value || typeof value !== 'object') return visible;
	for (const id of HIDEABLE_COLUMNS) {
		const flag = (value as Record<string, unknown>)[id];
		if (typeof flag === 'boolean') visible[id] = flag;
	}
	return visible;
}
