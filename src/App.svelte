<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { ServiceAction } from '$lib/queue';
	import type {
		ServiceConfigChanged,
		ServiceInfo,
		ServiceStartType,
		ServiceStatusChanged,
		ServicesChanged
	} from '$lib/tauri/bindings';
	import { loadServices, runServiceAction, updateStartupType } from '$lib/api/services';
	import { subscribeToLiveness } from '$lib/api/liveness';
	import { formatApiError } from '$lib/api/errors';
	import { isErr } from '$lib/result';
	import {
		applyConfigChanged,
		applyOptimisticStartType,
		applyServicesChanged,
		applySnapshot,
		applyStatusChanged,
		filterServices,
		revertStartType
	} from '$lib/state/services';
	import {
		createQueueState,
		dismiss,
		enqueue,
		pendingActions,
		scheduleSuccessDismiss,
		settle,
		shouldAutoDismiss
	} from '$lib/state/queue';
	import ActionQueue from '$lib/components/ActionQueue.svelte';
	import ElevationBanner from '$lib/components/ElevationBanner.svelte';
	import Toolbar from '$lib/components/Toolbar.svelte';
	import ServiceTable from '$lib/components/ServiceTable/ServiceTable.svelte';

	let services: ServiceInfo[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let query = $state('');
	let queue = $state(createQueueState());
	let unlisteners: Array<() => void> = [];

	/** Pending auto-dismiss timers for settled success items. */
	const autoDismissCancels: Array<() => void> = [];

	/** Actions currently in flight, per service name — drives the row spinners. */
	const pending = $derived(pendingActions(queue.items));

	const filtered = $derived(filterServices(services, query));

	function runAction(name: string, action: ServiceAction) {
		const { state, id } = enqueue(queue, { serviceName: name, action });
		queue = state;
		runServiceAction(name, action).then((result) => {
			if (isErr(result)) {
				queue = settle(queue, id, { status: 'failed', error: formatApiError(result.error) });
			} else {
				queue = settle(queue, id, { status: 'success' });
				scheduleDismiss(id);
			}
		});
	}

	/** Optimistically sets a service's startup type; reverts if the change fails. */
	function runStartupChange(name: string, startType: ServiceStartType) {
		const { state, id } = enqueue(queue, { serviceName: name, action: 'setStartType', startType });
		queue = state;
		const optimistic = applyOptimisticStartType(services, name, startType);
		services = optimistic.next;
		updateStartupType(name, startType).then((result) => {
			if (isErr(result)) {
				queue = settle(queue, id, { status: 'failed', error: formatApiError(result.error) });
				services = revertStartType(services, name, startType, optimistic.previous);
			} else {
				queue = settle(queue, id, { status: 'success' });
				scheduleDismiss(id);
			}
		});
	}

	function dismissItem(id: number) {
		queue = dismiss(queue, id);
	}

	/** Success items auto-clear after a short delay; failures persist until dismissed. */
	function scheduleDismiss(id: number) {
		const item = queue.items.find((entry) => entry.id === id);
		if (item && shouldAutoDismiss(item)) {
			autoDismissCancels.push(scheduleSuccessDismiss(id, dismissItem));
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
		await load();
	});

	onDestroy(() => {
		for (const cancel of autoDismissCancels) cancel();
		for (const unlisten of unlisteners) unlisten();
	});
</script>

<main class="page">
	<ElevationBanner />
	<Toolbar bind:value={query} />

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
			onAction={runAction}
			onStartupChange={runStartupChange}
		/>
	{/if}
</main>

<ActionQueue items={queue.items} onDismiss={dismissItem} />

<style>
	.page {
		max-width: 1100px;
		margin: 0 auto;
		padding: 16px 20px 120px;
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
