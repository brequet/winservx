<script lang="ts">
	import type { ServiceAction } from '$lib/api/services';
	import { ACTION_LABEL, type QueueAction } from '$lib/queue';
	import type {
		ServiceInfo,
		ServiceKind,
		ServiceStartType,
		ServiceState
	} from '$lib/tauri/bindings';
	import Select from '$lib/components/ui/Select.svelte';

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

	const STATE_LABEL: Record<ServiceState, string> = {
		running: 'running',
		stopped: 'stopped',
		startPending: 'starting',
		stopPending: 'stopping',
		continuePending: 'continuing',
		pausePending: 'pausing',
		paused: 'paused',
		unknown: 'unknown'
	};

	interface RowAction {
		action: ServiceAction;
		label: string;
		title?: string;
	}

	/** The actions valid for a service's current state; spec: only show what's actually possible. */
	function rowActions(service: ServiceInfo): RowAction[] {
		switch (service.state) {
			case 'running':
			case 'paused':
				return [
					{ action: 'stop', label: 'stop' },
					{ action: 'restart', label: 'restart' }
				];
			case 'stopped':
				return service.startType === 'disabled'
					? [
							{
								action: 'forceStart',
								label: 'force start',
								title: 'disabled — sets startup type to manual, then starts'
							}
						]
					: [{ action: 'start', label: 'start' }];
			default:
				return [];
		}
	}

	function stripeClass(state: ServiceState): string {
		switch (state) {
			case 'running':
				return 'stripe--running';
			case 'startPending':
			case 'stopPending':
			case 'continuePending':
			case 'pausePending':
				return 'stripe--pending';
			case 'paused':
				return 'stripe--error';
			default:
				return 'stripe--stopped';
		}
	}

	function statusClass(state: ServiceState): string {
		switch (state) {
			case 'running':
				return 'status--running';
			case 'startPending':
			case 'stopPending':
			case 'continuePending':
			case 'pausePending':
				return 'status--pending';
			case 'paused':
				return 'status--error';
			default:
				return 'status--stopped';
		}
	}

	function startupClass(startType: ServiceStartType | null): string {
		switch (startType) {
			case 'disabled':
				return 'startup--disabled';
			case 'boot':
			case 'system':
			case 'automatic':
				return 'startup--automatic';
			default:
				return 'startup--manual';
		}
	}

	const DRIVER_KINDS: ServiceKind[] = ['kernelDriver', 'fileSystemDriver', 'recognizerDriver'];

	/** The start types a service can be set to; boot/system only apply to drivers. */
	function startupOptions(kind: ServiceKind): { value: ServiceStartType; label: string }[] {
		const values: ServiceStartType[] = DRIVER_KINDS.includes(kind)
			? ['boot', 'system', 'automatic', 'manual', 'disabled']
			: ['automatic', 'manual', 'disabled'];
		return values.map((value) => ({ value, label: value }));
	}
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
							<span class="spinner" aria-hidden="true"></span>
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
							<span class="spinner" aria-hidden="true"></span>
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

	.spinner {
		display: inline-block;
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--line);
		border-top-color: var(--color-primary);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
</style>
