<script lang="ts">
	import { DropdownMenu as Bits } from 'bits-ui';
	import DropdownMenu from '$lib/components/ui/DropdownMenu.svelte';
	import { resolvedTheme, setThemeMode, theme, type ThemeMode } from '$lib/theme/theme.svelte';

	let open = $state(false);
	const resolved = $derived(resolvedTheme());

	const MODES: { value: ThemeMode; label: string }[] = [
		{ value: 'system', label: 'System' },
		{ value: 'light', label: 'Light' },
		{ value: 'dark', label: 'Dark' }
	];
</script>

<DropdownMenu
	{open}
	onOpenChange={(value) => (open = value)}
	triggerClass="btn btn--ghost btn--icon"
	ariaLabel="Theme: {resolved}"
>
	{#snippet trigger()}
		{#if resolved === 'dark'}
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
			</svg>
		{:else}
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="4" />
				<path
					d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
				/>
			</svg>
		{/if}
	{/snippet}
	{#snippet content()}
		{#each MODES as mode (mode.value)}
			<Bits.Item
				class={mode.value === theme.mode ? 'dd-item dd-item--active' : 'dd-item'}
				onSelect={() => setThemeMode(mode.value)}
			>
				{mode.label}
				{#if mode.value === theme.mode}
					<span class="dd-check" aria-hidden="true">✓</span>
				{/if}
			</Bits.Item>
		{/each}
	{/snippet}
</DropdownMenu>
