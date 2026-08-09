<script lang="ts">
	import { onMount } from 'svelte';
	import { isElevated, relaunchAsElevated } from '$lib/api/privilege';
	import { formatApiError } from '$lib/api/errors';
	import { isErr, isOk } from '$lib/result';

	let elevated = $state<boolean | null>(null);
	let relaunching = $state(false);
	let relaunchError: string | null = $state(null);

	onMount(async () => {
		const elevation = await isElevated();
		if (isOk(elevation)) elevated = elevation.value;
	});

	async function onRelaunch() {
		relaunching = true;
		relaunchError = null;
		const result = await relaunchAsElevated();
		relaunching = false;
		if (isErr(result)) relaunchError = formatApiError(result.error);
	}
</script>

{#if elevated === false}
	<div class="elevation-banner" role="status" aria-labelledby="elevation-warning-title">
		<div class="elevation-content">
			<strong id="elevation-warning-title" class="elevation-title">
				not running as administrator
			</strong>
			<span class="elevation-text">start/stop/restart may fail</span>
			{#if relaunchError}
				<span class="elevation-error" role="alert">{relaunchError}</span>
			{/if}
		</div>
		<button
			class="btn btn--secondary"
			onclick={onRelaunch}
			disabled={relaunching}
			aria-busy={relaunching}
		>
			{relaunching ? 'relaunching…' : 'relaunch as administrator'}
		</button>
	</div>
{/if}

<style>
	.elevation-banner {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 10px;
		padding: 7px 10px 7px 12px;
		border: 1px solid var(--line);
		border-left: 3px solid var(--color-warning);
		border-radius: 2px;
		font-size: 12px;
		color: var(--text);
		background: var(--color-warning-surface);
	}

	.elevation-content {
		display: flex;
		flex: 1 1 auto;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0 8px;
		min-width: 0;
	}

	.elevation-title {
		font-weight: 600;
		color: var(--color-warning);
	}

	.elevation-text {
		color: var(--text-dim);
	}

	.elevation-banner .btn {
		margin-left: auto;
	}

	.elevation-error {
		flex-basis: 100%;
		color: var(--color-danger);
	}

	@media (max-width: 600px) {
		.elevation-banner {
			align-items: flex-start;
			flex-wrap: wrap;
		}

		.elevation-banner .btn {
			width: 100%;
			margin-left: 0;
			justify-content: center;
		}
	}
</style>
