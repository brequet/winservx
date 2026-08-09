<script lang="ts">
	import { DropdownMenu as Bits } from 'bits-ui';
	import DropdownMenu from '$lib/components/ui/DropdownMenu.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { escapeAction } from './logic';
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
		startName: 'Log on as',
		pid: 'PID',
		actions: 'Actions'
	};

	function focusSearch() {
		input?.focus();
	}

	/** True when the key alone should start or extend a search query. */
	function isTypingKey(event: KeyboardEvent): boolean {
		if (event.ctrlKey || event.metaKey || event.altKey) return false;
		if (event.isComposing) return false;
		if (event.key.length !== 1) return false;
		return event.key !== ' ';
	}

	/** True when typing goes somewhere meaningful (inputs, menus, listboxes). */
	function isTypingTarget(target: Element | null): boolean {
		if (!target) return false;
		if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return true;
		if (target instanceof HTMLElement && target.isContentEditable) return true;
		return target.closest('[role="menu"], [role="listbox"]') !== null;
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key === '/') {
			if (
				document.activeElement !== input &&
				!(document.activeElement instanceof HTMLButtonElement)
			) {
				event.preventDefault();
				focusSearch();
			}
			return;
		}
		if (event.key === 'Escape') {
			const action = escapeAction({
				focusedInSearch: document.activeElement === input,
				hasValue: value !== '',
				columnsOpen,
				typingTarget: isTypingTarget(document.activeElement)
			});
			if (action === 'clear') value = '';
			else if (action === 'blur-search') input?.blur();
			else if (action === 'close-columns') columnsOpen = false;
			return;
		}
		if (!isTypingKey(event) || isTypingTarget(document.activeElement)) return;
		event.preventDefault();
		columnsOpen = false;
		value = event.key;
		focusSearch();
		queueMicrotask(() => input?.setSelectionRange(value.length, value.length));
	}}
/>

<div class="toolbar">
	<span class="px" aria-hidden="true">/</span>
	<input
		class="search-input"
		bind:this={input}
		bind:value
		placeholder="search name, display name, pid…"
	/>
	<span class="search-hint">esc clears · / focuses</span>
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
