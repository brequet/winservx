<script lang="ts">
	import { DropdownMenu as Bits } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		open,
		onOpenChange,
		align = 'end',
		side = 'bottom',
		triggerClass = 'btn btn--ghost',
		ariaLabel,
		trigger,
		content
	}: {
		open: boolean;
		onOpenChange: (open: boolean) => void;
		align?: 'start' | 'center' | 'end';
		side?: 'top' | 'right' | 'bottom' | 'left';
		triggerClass?: string;
		ariaLabel?: string;
		trigger: Snippet;
		content: Snippet;
	} = $props();
</script>

<Bits.Root {open} {onOpenChange}>
	<Bits.Trigger class={triggerClass} aria-label={ariaLabel}>
		{@render trigger()}
	</Bits.Trigger>
	<Bits.Portal>
		<Bits.Content class="dd-content" {align} {side}>
			{@render content()}
		</Bits.Content>
	</Bits.Portal>
</Bits.Root>
