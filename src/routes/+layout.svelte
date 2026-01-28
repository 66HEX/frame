<script lang="ts">
	let { children } = $props();
	import { onMount } from 'svelte';
	import './layout.css';
	import { type } from '@tauri-apps/plugin-os';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { loadWindowOpacity } from '$lib/services/settings';
	import { initI18n } from '$lib/i18n';

	let platform = $state<string | null>(null);

	const handleKeydown = (e: KeyboardEvent) => {
		if (e.key === 'Tab') {
			e.preventDefault();
		}
	};

	onMount(() => {
		initI18n();

		platform = type();

		loadWindowOpacity().then((val) => {
			themeStore.opacity = val;
		});

		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});
</script>

<div
	class="**:focus:ring-none flex h-screen flex-col overflow-hidden border-none bg-background select-none **:focus:outline-none"
	class:rounded-2xl={platform === 'macos'}
	style="background-color: color-mix(in srgb, var(--background), transparent {100 -
		themeStore.opacity}%)"
>
	<div class="relative flex-1">
		{@render children()}
	</div>
</div>
