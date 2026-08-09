<script lang="ts">
	import { ACTION_LABEL, runtimeAction, type ServiceAction } from '$lib/queue';
	import type { QueueTask, ServiceInfo, ServiceStartType } from '$lib/tauri/bindings';
	import Select from '$lib/components/ui/Select.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import ActionMenu from './ActionMenu.svelte';
	import {
		rowActions,
		KIND_LABEL,
		logonLabel,
		startupOptions,
		stripeClass,
		statusClass,
		startupClass,
		STATE_LABEL,
		isTransitioning,
		sortAfterClick,
		sortServices,
		type ColumnVisibility,
		type SortColumn,
		type SortState
	} from './logic';

	let {
		services,
		pending,
		sort,
		visible,
		onAction,
		onStartupChange,
		onSortChange
	}: {
		services: ServiceInfo[];
		/** Tasks in flight per service name; at most one per service. */
		pending: Map<string, QueueTask>;
		sort: SortState;
		visible: ColumnVisibility;
		onAction: (name: string, action: ServiceAction) => void;
		onStartupChange: (name: string, startType: ServiceStartType) => void;
		onSortChange: (sort: SortState) => void;
	} = $props();

	const sorted = $derived(sortServices(services, sort));

	function ariaSort(column: SortColumn): 'ascending' | 'descending' | undefined {
		if (sort?.column !== column) return undefined;
		return sort.direction === 'asc' ? 'ascending' : 'descending';
	}
</script>

{#snippet sortHeader(column: SortColumn, label: string, right = false)}
	<th scope="col" class:th-right={right} aria-sort={ariaSort(column)}>
		<button class="sort-btn" onclick={() => onSortChange(sortAfterClick(sort, column))}>
			{label}
			{#if sort?.column === column}
				<span class="sort-glyph sort-glyph--active" aria-hidden="true">
					{sort.direction === 'asc' ? '▲' : '▼'}
				</span>
			{:else}
				<span class="sort-glyph" aria-hidden="true">↕</span>
			{/if}
		</button>
	</th>
{/snippet}

<table class="table">
	<colgroup>
		<col class="col-stripe" />
		<col class="col-status" />
		{#if visible.displayName}
			<col class="col-display" />
		{/if}
		{#if visible.kind}
			<col class="col-kind" />
		{/if}
		<col class="col-tech" />
		{#if visible.startType}
			<col class="col-startup" />
		{/if}
		{#if visible.startName}
			<col class="col-logon" />
		{/if}
		{#if visible.pid}
			<col class="col-pid" />
		{/if}
		<col class="col-actions" />
	</colgroup>
	<thead>
		<tr>
			<th scope="col" aria-hidden="true"></th>
			{@render sortHeader('state', 'Status')}
			{#if visible.displayName}
				{@render sortHeader('displayName', 'Display name')}
			{/if}
			{#if visible.kind}
				<th scope="col">Kind</th>
			{/if}
			{@render sortHeader('name', 'Service name')}
			{#if visible.startType}
				{@render sortHeader('startType', 'Startup', true)}
			{/if}
			{#if visible.startName}
				<th scope="col">Log on as</th>
			{/if}
			{#if visible.pid}
				<th scope="col" class="th-right">PID</th>
			{/if}
			<th scope="col" class="th-right">Actions</th>
		</tr>
	</thead>
	<tbody>
		{#each sorted as service (service.name)}
			{@const rowPending = pending.get(service.name)}
			{@const rowAction = rowPending ? runtimeAction(rowPending.action) : null}
			<tr>
				<td class="td-stripe" aria-hidden="true">
					<span class="stripe {stripeClass(service.state)}"></span>
				</td>
				<td class="status {statusClass(service.state)}">{STATE_LABEL[service.state]}</td>
				{#if visible.displayName}
					<td class="display-name" title={service.displayName}>{service.displayName}</td>
				{/if}
				{#if visible.kind}
					<td class="kind" title={service.kind}>{KIND_LABEL[service.kind]}</td>
				{/if}
				<td class="tech-name" title={service.name}>{service.name}</td>
				{#if visible.startType}
					<td class="startup {startupClass(service.startType)}">
						{#if rowPending && !rowAction}
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
				{/if}
				{#if visible.startName}
					<td class="logon" title={service.startName ?? ''}>{logonLabel(service.startName)}</td>
				{/if}
				{#if visible.pid}
					<td class="pid">{service.pid ?? '—'}</td>
				{/if}
				<td class="actions">
					<div class="action-items">
						{#if rowAction}
							<span class="in-flight">
								<Spinner />
								{ACTION_LABEL[rowAction]}
							</span>
						{:else if isTransitioning(service.state)}
							<span class="transitioning">{STATE_LABEL[service.state]}…</span>
						{:else}
							{#each rowActions(service) as action (action.action)}
								<button
									class="btn btn--secondary action-btn"
									title={action.title}
									onclick={() => onAction(service.name, action.action)}
								>
									{action.label}
								</button>
							{/each}
						{/if}
						<ActionMenu {service} />
					</div>
				</td>
			</tr>
		{/each}
	</tbody>
</table>

<style>
	.table {
		width: 100%;
		min-width: 0;
		table-layout: fixed;
		border-collapse: separate;
		border-spacing: 0;
		margin-top: 2px;
	}

	.col-stripe {
		width: 18px;
	}

	.col-status {
		width: 110px;
	}

	.col-display {
		width: 220px;
	}

	.col-kind {
		width: 90px;
	}

	.col-tech {
		width: 210px;
	}

	.col-startup {
		width: 100px;
	}

	.col-logon {
		width: 110px;
	}

	.col-pid {
		width: 80px;
	}

	.col-actions {
		width: 160px;
	}

	thead {
		position: sticky;
		top: 0;
		z-index: 1;
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
		background: var(--surface);
		white-space: nowrap;
	}

	.th-right {
		text-align: right;
	}

	.sort-btn {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
	}

	.sort-btn:hover {
		color: var(--text);
	}

	.sort-glyph {
		display: inline-block;
		width: 9px;
		font-size: 8px;
		line-height: 1;
		text-align: center;
		opacity: 0.25;
	}

	.sort-btn:hover .sort-glyph {
		opacity: 0.8;
	}

	.sort-glyph--active {
		opacity: 1;
		color: var(--accent);
	}

	.sort-btn:hover .sort-glyph--active {
		opacity: 1;
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

	.kind {
		font-size: 11.5px;
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pid {
		text-align: right;
		font-size: 11px;
		color: var(--text-dim);
	}

	.logon {
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
		color: var(--startup-disabled);
	}

	.actions {
		text-align: right;
		white-space: nowrap;
	}

	.action-items {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
	}

	.action-btn {
		height: 20px;
		padding: 1px 8px;
		font-size: 11px;
	}

	.in-flight {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--status-pending);
	}

	.transitioning {
		font-size: 11px;
		color: var(--text-dim);
	}

	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
</style>
