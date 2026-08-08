<script lang="ts">
	import { commands, type GreetResponse } from './lib/tauri/bindings';

	let name = $state('');
	let greetMsg = $state('Press the button');

	let debugRes: GreetResponse | null = $state(null);

	async function greet(e: SubmitEvent) {
		e.preventDefault();
		debugRes = await commands.greet(name);
		greetMsg = debugRes.message;
	}
</script>

<h1>Welcome to Tauri</h1>

<div class="row">
	<a href="https://vite.dev" target="_blank">
		<img src="/src/assets/vite.svg" class="logo vite" alt="Vite logo" />
	</a>
	<a href="https://tauri.app" target="_blank">
		<img src="/src/assets/tauri.svg" class="logo tauri" alt="Tauri logo" />
	</a>
	<a href="https://svelte.dev" target="_blank">
		<img src="/src/assets/typescript.svg" class="logo typescript" alt="Svelte logo" />
	</a>
</div>

<p>Click on the Tauri logo to learn more about the framework</p>

<form class="row" onsubmit={greet}>
	<input id="greet-input" bind:value={name} placeholder="Enter a name..." />
	<button type="submit">Greet</button>
</form>
<p>{greetMsg}</p>

{#if debugRes !== null}
	<pre>{JSON.stringify(debugRes)}</pre>
{/if}
