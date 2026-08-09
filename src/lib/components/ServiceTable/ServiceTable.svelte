<script lang="ts">
	import type { ServiceAction } from '$lib/queue';
	import { ACTION_LABEL, type QueueAction } from '$lib/queue';
	import type { ServiceInfo, ServiceStartType } from '$lib/tauri/bindings';
	import Select from '$lib/components/ui/Select.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		rowActions,
		startupOptions,
		stripeClass,
		statusClass,
		startupClass,
		STATE_LABEL
	} from './logic';

	let {
		services,
		pending,
		onAction,
		onStartupChange
	}: {
		services: ServiceInfo[];
		/** Actions currently in flight per service name; at most one per service. */
		pending: Map<string, QueueAction>;
		onAction: (name: string, action: ServiceAction) => void;
		onStartupChange: (name: string, startType: ServiceStartType) => void;
	} = $props();
</script>

<table class="table">
	<colgroup>
		<col class="col-stripe" />
		<col class="col-status" />
		<col class="col-display" />
		<col class="col-tech" />
		<col class="col-startup" />
		<col class="col-actions" />
	</colgroup>
	<thead>
		<tr>
			<th scope="col" aria-hidden="true"></th>
			<th scope="col">Status</th>
			<th scope="col">Display name</th>
			<th scope="col">Service name</th>
			<th scope="col" class="th-right">Startup</th>
			<th scope="col" class="th-right">Actions</th>
		</tr>
	</thead>
	<tbody>
		{#each services as service (service.name)}
			{@const rowPending = pending.get(service.name)}
			<tr>
				<td class="td-stripe" aria-hidden="true">
					<span class="stripe {stripeClass(service.state)}"></span>
				</td>
				<td class="status {statusClass(service.state)}">{STATE_LABEL[service.state]}</td>
				<td class="display-name" title={service.displayName}>{service.displayName}</td>
				<td class="tech-name" title={service.name}>{service.name}</td>
				<td class="startup {startupClass(service.startType)}">
					{#if rowPending === 'setStartType'}
						<span class="in-flight">
							<Spinner />
						</span>
					{:else if service.startType && service.startType !== 'unknown'}
						<Select
							value={service.startType}
							onValueChange={(startType) =>
								onStartupChange(service.name, startType as ServiceStartType)}
							options={startupOptions(service.kind)}
							ariaLabel={`startup type of ${service.name}`}
						/>
					{:else}
						unknown
					{/if}
				</td>
				<td class="actions">
					{#if rowPending && rowPending !== 'setStartType'}
						<span class="in-flight">
							<Spinner />
							{ACTION_LABEL[rowPending]}
						</span>
					{:else}
						{#each rowActions(service) as rowAction (rowAction.action)}
							<button
								class="btn btn--ghost action-btn"
								title={rowAction.title}
								onclick={() => onAction(service.name, rowAction.action)}
							>
								{rowAction.label}
							</button>
						{/each}
					{/if}
				</td>
			</tr>
		{/each}
	</tbody>
</table>

<style>
	.table {
		width: 100%;
		table-layout: fixed;
		border-collapse: collapse;
		margin-top: 2px;
	}

	.col-stripe {
		width: 18px;
	}

	.col-status {
		width: 110px;
	}

	.col-tech {
		width: 210px;
	}

	.col-startup {
		width: 100px;
	}

	.col-actions {
		width: 150px;
	}

	thead tr {
		border-bottom: 2px solid var(--line-strong);
	}

	th {
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-dim);
		text-align: left;
		padding: 10px 2px 8px;
	}

	.th-right {
		text-align: right;
	}

	tbody tr {
		border-bottom: 1px solid var(--line);
	}

	tbody tr:hover {
		background: var(--surface-alt);
	}

	td {
		padding: 8px 2px;
		font-size: 12.5px;
		vertical-align: middle;
	}

	.td-stripe {
		padding-left: 0;
	}

	.stripe {
		display: inline-block;
		width: 3px;
		height: 20px;
	}

	.stripe--running {
		background: var(--status-running);
	}

	.stripe--stopped {
		border: 1px solid var(--line);
		height: 18px;
	}

	.stripe--pending {
		background: var(--status-pending);
		animation: blink 0.9s steps(2) infinite;
	}

	.stripe--error {
		background: var(--status-error);
	}

	.status {
		font-size: 11px;
		color: var(--status-stopped);
	}

	.status--running {
		color: var(--status-running);
		font-weight: 600;
	}

	.status--pending {
		color: var(--status-pending);
		font-weight: 600;
	}

	.status--error {
		color: var(--status-error);
	}

	.display-name {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tech-name {
		font-size: 11.5px;
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.startup {
		text-align: right;
		font-size: 11.5px;
		color: var(--status-stopped);
	}

	.startup--automatic {
		color: var(--text);
	}

	.startup--disabled {
		color: var(--status-error);
	}

	.actions {
		text-align: right;
		white-space: nowrap;
	}

	.action-btn {
		padding: 1px 8px;
		font-size: 11px;
		margin-left: 6px;
	}

	.in-flight {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--status-pending);
	}

	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
</style>
