<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { Plus, Play, FileVideo, HardDrive, LayoutList, Terminal } from 'lucide-svelte';
	import { cn } from '$lib/utils/cn';
	import frameIcon from '$lib/assets/icons/frame.svg?raw';

	const appWindow = getCurrentWindow();

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

	function minimize() {
		appWindow.minimize();
	}

	function close() {
		appWindow.close();
	}

	async function toggleMaximize() {
		const maximized = await appWindow.isMaximized();
		if (maximized) {
			await appWindow.unmaximize();
		} else {
			await appWindow.maximize();
		}
	}

	function formatTotalSize(bytes: number) {
		if (bytes === 0) return '0 KB';
		const mb = bytes / (1024 * 1024);
		return mb > 1000 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`;
	}
</script>

<div
	class="w-full pt-2 flex items-center justify-between px-4 select-none z-50 shrink-0"
	data-tauri-drag-region
>
	<div class="flex items-center gap-6 pointer-events-none mt-0.5">
		<div class="flex items-center z-50 mr-2 group pointer-events-auto">
			<button
				onclick={close}
				class="size-6 rounded-full flex items-center justify-center transition-opacity"
				title="Close"
			>
				<svg viewBox="-10 -10 20 20" class="w-full h-full" aria-hidden="true">
					<circle r="6" fill="#ff5f56" stroke="#e0443e" stroke-width="0.6" />
					<path
						d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8"
						stroke="#4a0002"
						stroke-width="1.5"
						stroke-linecap="round"
						class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
					/>
				</svg>
			</button>
			<button
				onclick={minimize}
				class="size-6 rounded-full flex items-center justify-center transition-opacity"
				title="Minimize"
			>
				<svg viewBox="-10 -10 20 20" class="w-full h-full" aria-hidden="true">
					<circle r="6" fill="#ffbd2e" stroke="#dea123" stroke-width="0.6" />
					<line
						x1="-2.4"
						y1="0"
						x2="2.4"
						y2="0"
						stroke="#5a3900"
						stroke-width="1.5"
						stroke-linecap="round"
						class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
					/>
				</svg>
			</button>
			<button
				onclick={toggleMaximize}
				class="size-6 rounded-full flex items-center justify-center transition-opacity"
				title="Toggle size"
			>
				<svg viewBox="-10 -10 20 20" class="w-full h-full" aria-hidden="true">
					<circle r="6" fill="#27c93f" stroke="#1aab29" stroke-width="0.6" />
					<g
						fill="#004200"
						class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
					>
						<path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z" />
						<path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z" />
					</g>
				</svg>
			</button>
		</div>

		<span
			class="flex items-center justify-center px-2 [&>svg]:size-5 [&>svg]:opacity-60 [&>svg]:fill-current text-foreground pointer-events-none"
			aria-hidden="true"
		>
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html frameIcon}
		</span>

		<div class="h-6 w-px bg-gray-alpha-100 pointer-events-none"></div>

		{#if onChangeView}
			<div
				class="flex items-center gap-1 bg-gray-alpha-100 p-0.5 h-7 rounded border border-gray-alpha-100 pointer-events-auto"
			>
				<button
					onclick={() => onChangeView('dashboard')}
					class={cn(
						'flex items-center gap-2 px-3 py-1 rounded-xs text-[10px]  font-medium transition-all uppercase tracking-wide',
						activeView === 'dashboard'
							? 'bg-foreground text-black shadow-sm'
							: 'text-gray-alpha-600 hover:text-foreground'
					)}
				>
					<LayoutList size={12} />
					<span>Dashboard</span>
				</button>
				<button
					onclick={() => onChangeView('logs')}
					class={cn(
						'flex items-center gap-2 px-3 py-1 rounded-xs text-[10px] font-medium transition-all uppercase tracking-wide',
						activeView === 'logs'
							? 'bg-foreground text-black shadow-sm'
							: 'text-gray-alpha-600 hover:text-foreground'
					)}
				>
					<Terminal size={12} />
					<span>Logs</span>
				</button>
			</div>
		{/if}

		<div class="h-6 w-px bg-gray-alpha-100 pointer-events-none"></div>

		<div class="flex items-center gap-4 text-[10px] text-gray-alpha-600 pointer-events-none">
			<div class="flex items-center gap-2">
				<HardDrive size={12} />
				<span>STORAGE: {formatTotalSize(totalSize)}</span>
			</div>
			<div class="flex items-center gap-2">
				<FileVideo size={12} />
				<span>ITEMS: {fileCount}</span>
			</div>
		</div>
	</div>

	<div class="flex items-center gap-3 mt-0.5 pointer-events-none">
		{#if onAddFile}
			<button
				onclick={onAddFile}
				class="flex items-center gap-2 bg-gray-alpha-100 hover:bg-gray-alpha-200 h-7 text-foreground px-3 py-1.5 rounded text-[10px] font-medium transition-colors cursor-pointer border border-gray-alpha-100 uppercase tracking-wide pointer-events-auto"
			>
				<Plus size={12} />
				Add Source
			</button>
		{/if}

		{#if onStartConversion}
			<button
				onclick={onStartConversion}
				disabled={isProcessing || fileCount === 0}
				class={cn(
					'flex items-center gap-2 px-4 py-1.5 rounded text-[10px] h-7 font-medium uppercase tracking-wide transition-all bg-foreground text-black hover:bg-foreground border border-foreground pointer-events-auto',
					(isProcessing || fileCount === 0) && 'opacity-50 cursor-not-allowed'
				)}
			>
				{#if isProcessing}
					<span class="animate-pulse">PROCESSING...</span>
				{:else}
					<Play size={12} fill="currentColor" />
					START
				{/if}
			</button>
		{/if}
	</div>
</div>
