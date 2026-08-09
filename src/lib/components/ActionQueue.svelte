<script lang="ts">
	import { actionLabel } from '$lib/queue';
	import type { QueueTask } from '$lib/tauri/bindings';
	import { formatApiError } from '$lib/api/errors';
	import Spinner from '$lib/components/ui/Spinner.svelte';

	let { items, onDismiss }: { items: QueueTask[]; onDismiss: (id: number) => void } = $props();
</script>

{#if items.length > 0}
	<aside class="queue" aria-label="action queue">
		<ul class="queue-list">
			{#each [...items].reverse() as item (item.id)}
				<li class="queue-item" class:queue-item--failed={item.status === 'failed'}>
					<span class="queue-id">#{item.id}</span>
					<span class="queue-action">{actionLabel(item.action)}</span>
					<span class="queue-name" title={item.serviceName}>{item.serviceName}</span>
					{#if item.status === 'queued' || item.status === 'running'}
						<span class="queue-state">
							<Spinner />
							{item.status}
						</span>
					{:else if item.status === 'success'}
						<span class="queue-state">done</span>
					{:else}
						<span class="queue-error" title={item.error ? formatApiError(item.error) : undefined}>
							{item.error ? formatApiError(item.error) : 'failed'}
						</span>
						<button
							class="btn btn--ghost btn--icon queue-dismiss"
							aria-label="dismiss"
							onclick={() => onDismiss(item.id)}
						>
							×
						</button>
					{/if}
				</li>
			{/each}
		</ul>
	</aside>
{/if}

<style>
	.queue {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 50;
		max-height: 220px;
		overflow-y: auto;
		padding: 6px 20px 8px;
		background: var(--surface);
		border-top: 1px solid var(--line-strong);
		font-size: 11px;
	}

	.queue-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.queue-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 2px 0;
	}

	.queue-id {
		color: var(--text-dim);
	}

	.queue-action {
		color: var(--text-dim);
	}

	.queue-name {
		font-weight: 600;
		max-width: 260px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.queue-item--failed .queue-name {
		color: var(--color-danger);
	}

	.queue-state {
		margin-left: auto;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--text-dim);
		white-space: nowrap;
	}

	.queue-error {
		flex: 1;
		text-align: right;
		color: var(--color-danger);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.queue-dismiss {
		flex-shrink: 0;
	}
</style>
