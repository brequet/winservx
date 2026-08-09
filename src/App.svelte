<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { ServiceAction } from '$lib/api/services';
	import type {
		ServiceConfigChanged,
		ServiceInfo,
		ServiceStartType,
		ServiceStatusChanged,
		ServicesChanged
	} from '$lib/tauri/bindings';
	import { loadServices, runServiceAction, updateStartupType } from '$lib/api/services';
	import { isElevated, relaunchAsElevated } from '$lib/api/privilege';
	import { subscribeToLiveness } from '$lib/api/liveness';
	import { formatApiError } from '$lib/api/errors';
	import { isErr, isOk } from '$lib/result';
	import type { QueueItem } from '$lib/queue';
	import ActionQueue from '$lib/components/ActionQueue.svelte';
	import ServiceTable from '$lib/components/ServiceTable.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let services: ServiceInfo[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let query = $state('');
	let searchInput: HTMLInputElement | undefined = $state();
	let queue: QueueItem[] = $state([]);
	let nextQueueId = $state(1);
	let elevated = $state<boolean | null>(null);
	let relaunching = $state(false);
	let relaunchError: string | null = $state(null);
	let unlisteners: Array<() => void> = [];

	/** Actions currently in flight, per service name — drives the row spinners. */
	const pending = $derived(
		new Map(
			queue
				.filter((item) => item.status === 'inFlight')
				.map((item) => [item.serviceName, item.action])
		)
	);

	/** Success items clear automatically after a short delay; failures persist until dismissed. */
	const SUCCESS_CLEAR_MS = 2000;

	function runAction(name: string, action: ServiceAction) {
		const id = nextQueueId++;
		queue = [{ id, serviceName: name, action, status: 'inFlight' }, ...queue];
		runServiceAction(name, action).then((result) => {
			if (isErr(result)) {
				settle(id, { status: 'failed', error: formatApiError(result.error) });
			} else {
				settle(id, { status: 'success' });
				setTimeout(() => dismiss(id), SUCCESS_CLEAR_MS);
			}
		});
	}

	/** Optimistically sets a service's startup type; reverts if the change fails. */
	function runStartupChange(name: string, startType: ServiceStartType) {
		const id = nextQueueId++;
		queue = [
			{ id, serviceName: name, action: 'setStartType', startType, status: 'inFlight' },
			...queue
		];
		const service = services.find((s) => s.name === name);
		const previous = service?.startType ?? null;
		if (service) service.startType = startType;
		updateStartupType(name, startType).then((result) => {
			if (isErr(result)) {
				settle(id, { status: 'failed', error: formatApiError(result.error) });
				if (service?.startType === startType) service.startType = previous;
			} else {
				settle(id, { status: 'success' });
				setTimeout(() => dismiss(id), SUCCESS_CLEAR_MS);
			}
		});
	}

	/** Applies a patch to a queue item; immutable replacement so reactivity fires. */
	function settle(id: number, patch: Partial<QueueItem>) {
		queue = queue.map((item) => (item.id === id ? { ...item, ...patch } : item));
	}

	function dismiss(id: number) {
		queue = queue.filter((item) => item.id !== id);
	}

	const filtered = $derived(
		query.trim() === ''
			? services
			: services.filter((service) => {
					const needle = query.trim().toLowerCase();
					return (
						service.name.toLowerCase().includes(needle) ||
						service.displayName.toLowerCase().includes(needle) ||
						String(service.pid ?? '').includes(needle)
					);
				})
	);

	function upsert(service: ServiceInfo) {
		const index = services.findIndex((s) => s.name === service.name);
		if (index === -1) {
			services = [...services, service];
		} else {
			services[index] = service;
		}
	}

	function onStatusChanged(event: ServiceStatusChanged) {
		const service = services.find((s) => s.name === event.name);
		if (service) {
			service.state = event.state;
			service.pid = event.pid;
		}
	}

	function onConfigChanged(event: ServiceConfigChanged) {
		const service = services.find((s) => s.name === event.name);
		if (service) {
			service.displayName = event.displayName;
			service.startType = event.startType;
		}
	}

	function onServicesChanged(event: ServicesChanged) {
		if (event.removed.length > 0) {
			const removed = new Set(event.removed);
			services = services.filter((s) => !removed.has(s.name));
		}
		for (const service of event.added) upsert(service);
	}

	async function load() {
		loading = true;
		error = null;
		const result = await loadServices();
		if (isErr(result)) {
			error = formatApiError(result.error);
		} else {
			services = result.value;
		}
		loading = false;
	}

	function focusSearch() {
		searchInput?.focus();
	}

	async function onRelaunch() {
		relaunching = true;
		relaunchError = null;
		const result = await relaunchAsElevated();
		relaunching = false;
		if (isErr(result)) relaunchError = formatApiError(result.error);
	}

	onMount(async () => {
		focusSearch();
		const elevation = await isElevated();
		if (isOk(elevation)) elevated = elevation.value;
		unlisteners = await subscribeToLiveness({
			onStatusChanged,
			onConfigChanged,
			onServicesChanged
		});
		await load();
	});

	onDestroy(() => {
		for (const unlisten of unlisteners) unlisten();
	});
</script>

<svelte:window
	onkeydown={(event) => {
		if (
			event.key === '/' &&
			document.activeElement !== searchInput &&
			!(document.activeElement instanceof HTMLButtonElement)
		) {
			event.preventDefault();
			focusSearch();
		}
	}}
/>

<main class="page">
	{#if elevated === false}
		<div class="elevation-banner" role="note">
			<span class="elevation-text">
				running without administrator rights — start/stop/restart may fail
			</span>
			{#if relaunchError}
				<span class="elevation-error">{relaunchError}</span>
			{/if}
			<button class="btn btn--primary" onclick={onRelaunch} disabled={relaunching}>
				{relaunching ? 'relaunching…' : 'relaunch as administrator'}
			</button>
		</div>
	{/if}
	<div class="toolbar">
		<span class="px" aria-hidden="true">/</span>
		<input
			class="search-input"
			bind:this={searchInput}
			bind:value={query}
			placeholder="search name, display name, pid…"
			onkeydown={(event) => {
				if (event.key === 'Escape') query = '';
			}}
		/>
		<span class="search-hint">esc to clear</span>
		<ThemeToggle />
	</div>

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

<ActionQueue items={queue} onDismiss={dismiss} />

<style>
	.page {
		max-width: 1100px;
		margin: 0 auto;
		padding: 16px 20px 120px;
	}

	.toolbar {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 2px 10px;
		border-bottom: 2px solid var(--line-strong);
	}

	.elevation-banner {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 10px;
		padding: 8px 12px;
		border: 1px solid var(--color-primary);
		border-radius: 2px;
		font-size: 12px;
		color: var(--text);
		background: var(--surface-alt);
	}

	.elevation-banner .btn {
		margin-left: auto;
	}

	.elevation-error {
		color: var(--color-danger);
	}

	.px {
		font-size: 12px;
		color: var(--text-dim);
	}

	.search-input {
		flex: 1;
		min-width: 0;
		font: inherit;
		font-size: 13px;
		color: var(--text);
		background: transparent;
		border: none;
		outline: none;
		padding: 2px 0;
	}

	.search-input::placeholder {
		color: var(--text-dim);
	}

	.search-hint {
		font-size: 10px;
		color: var(--text-dim);
		border: 1px solid var(--line);
		padding: 1px 5px;
		white-space: nowrap;
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
