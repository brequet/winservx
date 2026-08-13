<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { ServiceAction } from '$lib/queue';
	import type {
		QueueTask,
		QueueTaskUpdated,
		ServiceConfigChanged,
		ServiceInfo,
		ServiceStartType,
		ServiceStatusChanged,
		ServicesChanged
	} from '$lib/tauri/bindings';
	import { loadServices } from '$lib/api/services';
	import { dismissTask, enqueueAction, loadQueue, subscribeToQueue } from '$lib/api/queue';
	import { subscribeToLiveness } from '$lib/api/liveness';
	import { formatApiError } from '$lib/api/errors';
	import { isErr } from '$lib/result';
	import {
		applyConfigChanged,
		applyOptimisticStartType,
		applyServicesChanged,
		applySnapshot,
		applyStatusChanged,
		discardOptimisticStartType,
		filterServices,
		recordOptimisticStartType,
		revertStartType,
		settleOptimisticStartType,
		type OptimisticStartTypes
	} from '$lib/state/services';
	import {
		applyQueueSnapshot,
		applyTaskChanged,
		createQueueState,
		dismiss,
		insertPendingTask,
		pendingActions,
		scheduleSuccessDismiss,
		shouldAutoDismiss
	} from '$lib/state/queue';
	import { loadTablePrefs, saveTablePrefs } from '$lib/state/tablePrefs';
	import type { ColumnId, ColumnVisibility, SortState } from '$lib/components/ServiceTable/logic';
	import ActionQueue from '$lib/components/ActionQueue.svelte';
	import ElevationBanner from '$lib/components/ElevationBanner.svelte';
	import Toolbar from '$lib/components/Toolbar/Toolbar.svelte';
	import ServiceTable from '$lib/components/ServiceTable/ServiceTable.svelte';

	const prefs = loadTablePrefs();
	let services: ServiceInfo[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let query = $state('');
	let sort = $state<SortState>(prefs.sort);
	let visible = $state<ColumnVisibility>(prefs.visible);
	let queue = $state<QueueTask[]>(createQueueState());
	let unlisteners: Array<() => void> = [];
	let contentEl: HTMLElement | undefined = $state();
	let previousQuery = '';

	/**
	 * Optimistic startup-type changes, keyed by service name and recorded
	 * synchronously. Settled by `settleOptimisticStartType` on task events.
	 */
	let optimisticStartTypes: OptimisticStartTypes = new Map();

	$effect(() => {
		saveTablePrefs(sort, visible);
	});

	/** Scrolls back to the top only when the search query actually changes. */
	$effect(() => {
		const current = query;
		if (current !== previousQuery) {
			previousQuery = current;
			contentEl?.scrollTo({ top: 0 });
		}
	});

	/** Pending auto-dismiss timers for settled success items. */
	const autoDismissCancels: Array<() => void> = [];

	/** Tasks in flight per service name — drives the row spinners. */
	const pending = $derived(pendingActions(queue));

	const filtered = $derived(filterServices(services, query));

	function runAction(name: string, action: ServiceAction) {
		enqueueAction(name, action).then((result) => {
			if (isErr(result)) {
				console.error('failed to enqueue action', result.error);
				return;
			}
			queue = insertPendingTask(queue, {
				id: result.value,
				serviceName: name,
				action,
				status: 'queued',
				error: null
			});
		});
	}

	/** Optimistically sets a service's startup type; reverts when its task fails. */
	function runStartupChange(name: string, startType: ServiceStartType) {
		const optimistic = applyOptimisticStartType(services, name, startType);
		services = optimistic.next;
		// Record synchronously: a task can settle (and fail) before the enqueue
		// invoke resolves, so the entry must exist before its events arrive.
		optimisticStartTypes = recordOptimisticStartType(
			optimisticStartTypes,
			name,
			startType,
			optimistic.previous
		);
		enqueueAction(name, { setStartType: startType }).then((result) => {
			if (isErr(result)) {
				services = revertStartType(services, name, startType, optimistic.previous);
				optimisticStartTypes = discardOptimisticStartType(optimisticStartTypes, name);
				console.error('failed to enqueue startup change', result.error);
				return;
			}
			queue = insertPendingTask(queue, {
				id: result.value,
				serviceName: name,
				action: { setStartType: startType },
				status: 'queued',
				error: null
			});
		});
	}

	function dismissItem(id: number) {
		queue = dismiss(queue, id);
		void dismissTask(id);
	}

	function onSortChange(next: SortState) {
		sort = next;
	}

	function onColumnVisibilityChange(id: ColumnId, checked: boolean) {
		visible = { ...visible, [id]: checked };
		if (sort && !checked && sort.column === id) sort = null;
	}

	/** Success items auto-clear after a short delay; failures persist until dismissed. */
	function scheduleDismiss(id: number) {
		const item = queue.find((entry) => entry.id === id);
		if (item && shouldAutoDismiss(item)) {
			autoDismissCancels.push(scheduleSuccessDismiss(id, dismissItem));
		}
	}

	/** Patches the queue from backend task events; settles optimistic startup changes. */
	function onTaskUpdated(event: QueueTaskUpdated) {
		const { task } = event;
		queue = applyTaskChanged(queue, task);
		const settled = settleOptimisticStartType(services, optimisticStartTypes, task);
		services = settled.next;
		optimisticStartTypes = settled.entries;
		if (task.status === 'success') {
			scheduleDismiss(task.id);
		}
	}

	function onStatusChanged(event: ServiceStatusChanged) {
		services = applyStatusChanged(services, event);
	}

	function onConfigChanged(event: ServiceConfigChanged) {
		services = applyConfigChanged(services, event);
	}

	function onServicesChanged(event: ServicesChanged) {
		services = applyServicesChanged(services, event);
	}

	async function load() {
		loading = true;
		error = null;
		const result = await loadServices();
		if (isErr(result)) {
			error = formatApiError(result.error);
		} else {
			services = applySnapshot(services, result.value);
		}
		loading = false;
	}

	onMount(async () => {
		unlisteners = await subscribeToLiveness({
			onStatusChanged,
			onConfigChanged,
			onServicesChanged
		});
		unlisteners = [...unlisteners, ...(await subscribeToQueue({ onTaskUpdated }))];
		await load();
		const queueResult = await loadQueue();
		if (!isErr(queueResult)) {
			queue = applyQueueSnapshot(queue, queueResult.value);
		}
	});

	onDestroy(() => {
		for (const cancel of autoDismissCancels) cancel();
		for (const unlisten of unlisteners) unlisten();
	});
</script>

<main class="layout">
	<ElevationBanner />
	<Toolbar bind:value={query} {visible} {onColumnVisibilityChange} />

	<div class="content" bind:this={contentEl}>
		{#if error}
			<div class="error" role="alert">
				<div>
					<span class="error-label">failed to load services:</span>
					<span>{error}</span>
				</div>
				<button class="btn btn--ghost" onclick={load}>retry</button>
			</div>
		{:else if loading}
			<p class="hint">loading services…</p>
		{:else if filtered.length === 0}
			<p class="hint">
				{services.length === 0 ? 'no services found' : 'no services match the search'}
			</p>
		{:else}
			<ServiceTable
				services={filtered}
				{pending}
				{sort}
				{visible}
				onAction={runAction}
				onStartupChange={runStartupChange}
				{onSortChange}
			/>
		{/if}
	</div>
</main>

<ActionQueue items={queue} onDismiss={dismissItem} />

<style>
	.layout {
		display: flex;
		flex-direction: column;
		height: 100vh;
		max-width: 1100px;
		margin: 0 auto;
		padding: 16px 20px 0;
	}

	.content {
		flex: 1;
		min-height: 0;
		overflow-x: auto;
		overflow-y: auto;
		/* eslint-disable-next-line css/use-baseline -- supported in all Tauri webviews */
		overscroll-behavior: none;
		padding-bottom: 24px;
	}

	.hint {
		color: var(--text-dim);
		padding: 24px 2px;
	}

	.error {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-top: 2px;
		padding: 10px 12px;
		border: 1px solid var(--color-danger);
		border-radius: 2px;
		font-size: 12px;
		color: var(--color-danger);
		background: var(--surface-alt);
	}

	.error-label {
		font-weight: 600;
		margin-right: 8px;
	}
</style>
