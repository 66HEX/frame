<script lang="ts">
	import { onMount } from 'svelte';
	import { type } from '@tauri-apps/plugin-os';
	import MacosTitlebar from './titlebar/MacosTitlebar.svelte';
	import WindowsTitlebar from './titlebar/WindowsTitlebar.svelte';

	let {
		totalSize = 0,
		fileCount = 0,
		isProcessing = false,
		activeView = 'dashboard',
		onAddFile,
		onStartConversion,
		onChangeView
	}: {
		totalSize?: number;
		fileCount?: number;
		isProcessing?: boolean;
		activeView?: 'dashboard' | 'logs';
		onAddFile?: () => void;
		onStartConversion?: () => void;
		onChangeView?: (view: 'dashboard' | 'logs') => void;
	} = $props();

	let platform = $state<string | null>(null);

	onMount(() => {
		platform = type();
	});
</script>

{#if platform === 'windows'}
	<WindowsTitlebar
		{totalSize}
		{fileCount}
		{isProcessing}
		{activeView}
		{onAddFile}
		{onStartConversion}
		{onChangeView}
	/>
{:else}
	<MacosTitlebar
		{totalSize}
		{fileCount}
		{isProcessing}
		{activeView}
		{onAddFile}
		{onStartConversion}
		{onChangeView}
	/>
{/if}
