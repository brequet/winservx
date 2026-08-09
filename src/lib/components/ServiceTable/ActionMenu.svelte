<script lang="ts">
	import { onDestroy } from 'svelte';
	import { DropdownMenu as Bits } from 'bits-ui';
	import type { ServiceInfo } from '$lib/tauri/bindings';
	import DropdownMenu from '$lib/components/ui/DropdownMenu.svelte';
	import { copyItems, type CopyItem, type CopyItemId } from './logic';

	const COPIED_CLEAR_MS = 2000;

	let { service }: { service: ServiceInfo } = $props();

	let open = $state(false);
	let copied = $state<CopyItemId | null>(null);
	let copiedTimer: ReturnType<typeof setTimeout> | undefined;
	const items = $derived(copyItems(service));

	function clearCopied() {
		if (copiedTimer !== undefined) {
			clearTimeout(copiedTimer);
			copiedTimer = undefined;
		}
		copied = null;
	}

	function showCopied(id: CopyItemId) {
		clearCopied();
		copied = id;
		copiedTimer = setTimeout(() => {
			copied = null;
			copiedTimer = undefined;
		}, COPIED_CLEAR_MS);
	}

	async function copy(item: CopyItem) {
		try {
			await navigator.clipboard.writeText(item.text);
			showCopied(item.id);
		} catch (error) {
			console.error('failed to copy to clipboard', error);
		}
	}

	onDestroy(clearCopied);
</script>

<DropdownMenu
	{open}
	onOpenChange={(value) => {
		if (value) clearCopied();
		open = value;
	}}
	triggerClass="btn btn--ghost action-menu-btn"
	ariaLabel={`more actions for ${service.name}`}
>
	{#snippet trigger()}
		<span aria-hidden="true">⋮</span>
	{/snippet}
	{#snippet content()}
		{#each items as item (item.id)}
			<Bits.Item class="dd-item" onSelect={() => copy(item)} closeOnSelect={false}>
				{item.id === copied ? 'Copied ✓' : item.label}
			</Bits.Item>
		{/each}
	{/snippet}
</DropdownMenu>

<style>
	:global(.action-menu-btn) {
		width: 22px;
		height: 20px;
		padding: 1px 2px;
		justify-content: center;
		font-size: 11px;
		line-height: 1.4;
	}
</style>
