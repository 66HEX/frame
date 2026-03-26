<script lang="ts">
	let { children } = $props();
	import { onMount } from 'svelte';
	import './layout.css';
	import { type } from '@tauri-apps/plugin-os';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { loadFontFamily } from '$lib/services/settings';
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

		loadFontFamily().then((val) => {
			themeStore.fontFamily = val;
		});

		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});

	$effect(() => {
		const root = document.documentElement;
		if (themeStore.fontFamily === 'sans') {
			root.style.setProperty('--app-font-family', 'var(--font-archivo)');
		} else {
			root.style.setProperty('--app-font-family', 'var(--font-loskeley-mono)');
		}
	});
</script>

<div
	class="**:focus:ring-none relative flex h-screen flex-col overflow-hidden border-none bg-background select-none **:focus:outline-none"
	class:rounded-2xl={platform === 'macos'}
>
	<div class="relative flex-1">
		{@render children()}
	</div>
</div>
