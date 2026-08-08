<script lang="ts">
	import type { ServiceInfo, ServiceStartType, ServiceState } from '$lib/tauri/bindings';

	let { services }: { services: ServiceInfo[] } = $props();

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
</script>

<table class="table">
	<colgroup>
		<col class="col-stripe" />
		<col class="col-status" />
		<col class="col-display" />
		<col class="col-tech" />
		<col class="col-startup" />
	</colgroup>
	<thead>
		<tr>
			<th scope="col" aria-hidden="true"></th>
			<th scope="col">Status</th>
			<th scope="col">Display name</th>
			<th scope="col">Service name</th>
			<th scope="col" class="th-right">Startup</th>
		</tr>
	</thead>
	<tbody>
		{#each services as service (service.name)}
			<tr>
				<td class="td-stripe" aria-hidden="true">
					<span class="stripe {stripeClass(service.state)}"></span>
				</td>
				<td class="status {statusClass(service.state)}">{STATE_LABEL[service.state]}</td>
				<td class="display-name" title={service.displayName}>{service.displayName}</td>
				<td class="tech-name" title={service.name}>{service.name}</td>
				<td class="startup {startupClass(service.startType)}">
					{service.startType ?? 'unknown'}
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

	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
</style>
