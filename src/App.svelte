<script lang="ts">
	import { onMount } from 'svelte';
	import type { ServiceInfo } from '$lib/tauri/bindings';
	import { loadServices } from '$lib/api/services';
	import { formatApiError } from '$lib/api/errors';
	import { isErr } from '$lib/result';
	import ServiceTable from '$lib/components/ServiceTable.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let services: ServiceInfo[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let query = $state('');
	let searchInput: HTMLInputElement | undefined = $state();

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

	onMount(load);
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
		<button class="btn btn--ghost" onclick={load} disabled={loading}>
			{loading ? 'loading…' : 'refresh'}
		</button>
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
		<ServiceTable services={filtered} />
	{/if}
</main>

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
