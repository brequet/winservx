<script lang="ts">
	import type { ServiceInfo, ServiceState } from '../tauri/bindings';

	let { services }: { services: ServiceInfo[] } = $props();

	function stateLabel(state: ServiceState): string {
		return state.replace(/([A-Z])/g, ' $1').toLowerCase();
	}
</script>

<table>
	<thead>
		<tr>
			<th>Status</th>
			<th>Name</th>
			<th>PID</th>
		</tr>
	</thead>
	<tbody>
		{#each services as service (service.name)}
			<tr>
				<td>
					<span class="badge" class:running={service.state === 'running'}>
						{stateLabel(service.state)}
					</span>
				</td>
				<td>
					<span class="name">{service.displayName}</span>
					<span class="key">{service.name}</span>
				</td>
				<td>{service.pid ?? '—'}</td>
			</tr>
		{/each}
	</tbody>
</table>

<style>
	table {
		width: 100%;
		border-collapse: collapse;
		margin-top: 1rem;
	}

	th,
	td {
		text-align: left;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid #e0e0e0;
	}

	.badge {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: 999px;
		font-size: 0.8rem;
		background: #f0f0f0;
		color: #555;
	}

	.badge.running {
		background: #e3f4e3;
		color: #1b7f1b;
	}

	.name {
		display: block;
	}

	.key {
		display: block;
		font-size: 0.75rem;
		color: #888;
	}
</style>
