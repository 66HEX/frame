<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import {
		Plus,
		Play,
		FileVideo,
		HardDrive,
		LayoutList,
		Terminal,
		Minus,
		Square,
		X
	} from 'lucide-svelte';
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

<div class="w-full h-10 select-none z-50 shrink-0 relative" data-tauri-drag-region>
	<div class="absolute inset-0 px-4 flex items-center pointer-events-none">
		<div class="w-full grid grid-cols-12 gap-4">
			<div class="col-span-8 flex items-center gap-4 mt-4">
				<span
					class="flex items-center justify-center [&>svg]:size-5 [&>svg]:opacity-60 [&>svg]:fill-current text-foreground pointer-events-none"
					aria-hidden="true"
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html frameIcon}
				</span>

				<div class="h-5 w-px bg-gray-alpha-100 pointer-events-none"></div>

				{#if onChangeView}
					<div
						class="flex items-center gap-1 bg-gray-alpha-100 p-0.5 rounded border border-gray-alpha-100 pointer-events-auto"
					>
						<button
							onclick={() => onChangeView('dashboard')}
							class={cn(
								'flex items-center gap-2 px-3 py-1 rounded-xs text-[10px] font-medium transition-all uppercase tracking-wide',
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

				<div class="h-5 w-px bg-gray-alpha-100 pointer-events-none"></div>

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

			<div class="col-span-4 flex items-center gap-2 mt-4">
				{#if onAddFile}
					<button
						onclick={onAddFile}
						class="flex items-center gap-2 bg-gray-alpha-100 hover:bg-gray-alpha-200 text-foreground px-3 py-1.5 rounded text-[10px] font-medium transition-colors cursor-pointer border border-gray-alpha-100 uppercase tracking-wide pointer-events-auto"
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
							'flex items-center gap-2 px-4 py-1.5 rounded text-[10px] font-medium uppercase tracking-wide transition-all bg-foreground text-black hover:bg-foreground border border-foreground pointer-events-auto',
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
	</div>

	<div
		class="absolute right-0 top-0 h-full flex items-center pointer-events-auto border-2 border-transparent rounded-tr-2xl z-50"
	>
		<button
			onclick={minimize}
			class="h-full w-12 flex items-center justify-center hover:bg-gray-alpha-100 transition-colors text-foreground"
			title="Minimize"
		>
			<Minus size={16} />
		</button>
		<button
			onclick={toggleMaximize}
			class="h-full w-12 flex items-center justify-center hover:bg-gray-alpha-100 transition-colors text-foreground"
			title="Maximize"
		>
			<Square size={14} />
		</button>
		<button
			onclick={close}
			class="h-full w-12 flex items-center rounded-tr-[14px] justify-center hover:bg-ds-red-600 hover:text-foreground transition-colors text-foreground"
			title="Close"
		>
			<X size={16} />
		</button>
	</div>
</div>
