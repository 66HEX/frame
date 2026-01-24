<script lang="ts">
	let { children } = $props();
	import { onMount } from 'svelte';
	import './layout.css';
	import { type } from '@tauri-apps/plugin-os';

	let platform = $state<string | null>(null);

	onMount(() => {
		platform = type();

		const handleKeydown = (e: KeyboardEvent) => {
			if (e.key === 'Tab') {
				e.preventDefault();
			}
		};

		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});
</script>

<div
	class="**:focus:ring-none flex h-screen flex-col overflow-hidden border-none bg-background/80 select-none **:focus:outline-none"
	class:rounded-2xl={platform === 'macos'}
>
	<div class="relative flex-1">
		{@render children()}
	</div>
</div>
