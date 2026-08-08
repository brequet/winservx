<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	let name = $state('');
	let greetMsg = $state('Press the button');

	async function greet(e: SubmitEvent) {
		e.preventDefault();
		// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
		greetMsg = await invoke('greet', { name });
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
