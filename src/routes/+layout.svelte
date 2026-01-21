<script lang="ts">
	let { children } = $props();
	import { onMount } from 'svelte';
	import './layout.css';
	import { type } from '@tauri-apps/plugin-os';

	let platform = $state<string | null>(null);

	onMount(() => {
		platform = type();
	});
</script>

<div
	class="**:focus:ring-none flex h-screen flex-col overflow-hidden border-none bg-black/60 select-none **:focus:outline-none"
	class:rounded-2xl={platform === 'macos'}
>
	<div class="relative flex-1">
		{@render children()}
	</div>
</div>
