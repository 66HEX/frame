<script lang="ts">
	import { onMount } from 'svelte';
	import { type } from '@tauri-apps/plugin-os';
	import MacosTitlebar from './titlebar/MacosTitlebar.svelte';
	import WindowsTitlebar from './titlebar/WindowsTitlebar.svelte';
	import LinuxTitlebar from './titlebar/LinuxTitlebar.svelte';

	let {
		totalSize = 0,
		fileCount = 0,
		selectedCount = 0,
		isProcessing = false,
		activeView = 'dashboard',
		onAddFile,
		onStartConversion,
		onChangeView,
		onOpenSettings
	}: {
		totalSize?: number;
		fileCount?: number;
		selectedCount?: number;
		isProcessing?: boolean;
		activeView?: 'dashboard' | 'logs';
		onAddFile?: () => void;
		onStartConversion?: () => void;
		onChangeView?: (view: 'dashboard' | 'logs') => void;
		onOpenSettings?: () => void;
	} = $props();

	let platform = $state<string | null>(null);

	onMount(() => {
		platform = type();
	});
</script>

{#if platform === 'macos'}
	<MacosTitlebar
		{totalSize}
		{fileCount}
		{selectedCount}
		{isProcessing}
		{activeView}
		{onAddFile}
		{onStartConversion}
		{onChangeView}
		{onOpenSettings}
	/>
{:else if platform === 'windows'}
	<WindowsTitlebar
		{totalSize}
		{fileCount}
		{selectedCount}
		{isProcessing}
		{activeView}
		{onAddFile}
		{onStartConversion}
		{onChangeView}
		{onOpenSettings}
	/>
{:else}
	<LinuxTitlebar
		{totalSize}
		{fileCount}
		{selectedCount}
		{isProcessing}
		{activeView}
		{onAddFile}
		{onStartConversion}
		{onChangeView}
		{onOpenSettings}
	/>
{/if}
