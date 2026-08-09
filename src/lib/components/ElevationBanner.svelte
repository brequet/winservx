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
	<div class="elevation-banner" role="note">
		<span class="elevation-text">
			running without administrator rights — start/stop/restart may fail
		</span>
		{#if relaunchError}
			<span class="elevation-error">{relaunchError}</span>
		{/if}
		<button class="btn btn--primary" onclick={onRelaunch} disabled={relaunching}>
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
		padding: 8px 12px;
		border: 1px solid var(--color-primary);
		border-radius: 2px;
		font-size: 12px;
		color: var(--text);
		background: var(--surface-alt);
	}

	.elevation-banner .btn {
		margin-left: auto;
	}

	.elevation-error {
		color: var(--color-danger);
	}
</style>
