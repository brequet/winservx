<script lang="ts">
	import { DropdownMenu as Bits } from 'bits-ui';
	import DropdownMenu from '$lib/components/ui/DropdownMenu.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import {
		HIDEABLE_COLUMNS,
		type ColumnId,
		type ColumnVisibility
	} from '$lib/components/ServiceTable/logic';

	let {
		value = $bindable(''),
		visible,
		onColumnVisibilityChange
	}: {
		value: string;
		visible: ColumnVisibility;
		onColumnVisibilityChange: (id: ColumnId, checked: boolean) => void;
	} = $props();
	let input: HTMLInputElement | undefined = $state();
	let columnsOpen = $state(false);

	const COLUMN_LABELS: Record<ColumnId, string> = {
		stripe: 'Status bar',
		status: 'Status',
		displayName: 'Display name',
		kind: 'Kind',
		name: 'Service name',
		startType: 'Startup',
		pid: 'PID',
		actions: 'Actions'
	};

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
	<DropdownMenu
		open={columnsOpen}
		onOpenChange={(value) => (columnsOpen = value)}
		triggerClass="btn btn--ghost"
		ariaLabel="toggle column visibility"
	>
		{#snippet trigger()}
			columns
		{/snippet}
		{#snippet content()}
			{#each HIDEABLE_COLUMNS as id (id)}
				<Bits.CheckboxItem
					class="dd-item"
					checked={visible[id]}
					onCheckedChange={(checked) => onColumnVisibilityChange(id, checked)}
					closeOnSelect={false}
				>
					{#snippet children({ checked })}
						{COLUMN_LABELS[id]}
						{#if checked}
							<span class="dd-check" aria-hidden="true">✓</span>
						{/if}
					{/snippet}
				</Bits.CheckboxItem>
			{/each}
		{/snippet}
	</DropdownMenu>
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
