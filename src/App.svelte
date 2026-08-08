<script lang="ts">
	import { onMount } from 'svelte';
	import { type ServiceInfo } from './lib/tauri/bindings';
	import { formatApiError } from './lib/api/errors';
	import { loadServices } from './lib/api/services';
	import ServiceTable from './lib/components/ServiceTable.svelte';

	let services: ServiceInfo[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);

	async function load() {
		loading = true;
		error = null;
		const result = await loadServices();
		if (result.ok) {
			services = result.value;
		} else {
			error = formatApiError(result.error);
		}
		loading = false;
	}

	onMount(load);
</script>

<main class="page">
	<header class="header">
		<h1>WinServX</h1>
		<button onclick={load} disabled={loading}>
			{loading ? 'Loading…' : 'Refresh'}
		</button>
	</header>

	{#if error}
		<div class="error">Failed to load services: {error}</div>
	{:else if loading}
		<p class="hint">Loading services…</p>
	{:else if services.length === 0}
		<p class="hint">No services found.</p>
	{:else}
		<ServiceTable {services} />
	{/if}
</main>

<style>
	.page {
		max-width: 900px;
		margin: 0 auto;
		padding: 2rem;
		font-family: system-ui, sans-serif;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.hint {
		color: #888;
	}

	.error {
		color: #b00020;
		background: #ffeaea;
		border: 1px solid #f5c6c6;
		border-radius: 6px;
		padding: 0.75rem 1rem;
	}
</style>
