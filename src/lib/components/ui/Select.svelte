<script lang="ts">
	import { Select as Bits } from 'bits-ui';

	let {
		value,
		onValueChange,
		options,
		disabled = false,
		triggerClass = '',
		ariaLabel,
		side = 'bottom',
		align = 'end'
	}: {
		value: string;
		onValueChange: (value: string) => void;
		options: { value: string; label: string; disabled?: boolean }[];
		disabled?: boolean;
		triggerClass?: string;
		ariaLabel?: string;
		side?: 'top' | 'right' | 'bottom' | 'left';
		align?: 'start' | 'center' | 'end';
	} = $props();
</script>

<Bits.Root type="single" items={options} {value} {onValueChange} {disabled}>
	<Bits.Trigger class="select-trigger {triggerClass}" aria-label={ariaLabel}>
		<Bits.Value>
			{#snippet children({ selection })}
				<span class="select-value-text">
					{selection.type === 'single' ? (selection.selected?.label ?? value) : value}
				</span>
			{/snippet}
		</Bits.Value>
		<span class="select-caret" aria-hidden="true"></span>
	</Bits.Trigger>
	<Bits.Portal>
		<Bits.Content class="dd-content" {side} {align} sideOffset={4} preventScroll={false}>
			<Bits.Viewport class="select-viewport">
				{#each options as option (option.value)}
					<Bits.Item
						class="dd-item"
						value={option.value}
						label={option.label}
						disabled={option.disabled}
					>
						{#snippet children({ selected })}
							<span>{option.label}</span>
							{#if selected}
								<span class="dd-check" aria-hidden="true">✓</span>
							{/if}
						{/snippet}
					</Bits.Item>
				{/each}
			</Bits.Viewport>
		</Bits.Content>
	</Bits.Portal>
</Bits.Root>

<style>
	:global(.select-trigger) {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 2px 6px;
		font: inherit;
		line-height: inherit;
		color: inherit;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 2px;
		cursor: pointer;
	}

	:global(.select-trigger:hover) {
		border-color: var(--line);
	}

	:global(.select-trigger:focus-visible) {
		outline: 1px solid var(--color-focus);
		outline-offset: -1px;
	}

	.select-value-text {
		pointer-events: none;
	}

	.select-caret {
		width: 0;
		height: 0;
		border-left: 4px solid transparent;
		border-right: 4px solid transparent;
		border-top: 4px solid currentColor;
		opacity: 0.7;
	}
</style>
