<script lang="ts">
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { value = $bindable('') } = $props();
	let input: HTMLInputElement | undefined = $state();

	function focusSearch() {
		input?.focus();
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (
			event.key === '/' &&
			document.activeElement !== input &&
			!(document.activeElement instanceof HTMLButtonElement)
		) {
			event.preventDefault();
			focusSearch();
		}
	}}
/>

<div class="toolbar">
	<span class="px" aria-hidden="true">/</span>
	<input
		class="search-input"
		bind:this={input}
		bind:value
		placeholder="search name, display name, pid…"
		onkeydown={(event) => {
			if (event.key === 'Escape') value = '';
		}}
	/>
	<span class="search-hint">esc to clear</span>
	<ThemeToggle />
</div>

<style>
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
</style>
