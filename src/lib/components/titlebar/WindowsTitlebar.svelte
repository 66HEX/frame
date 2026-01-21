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
	import Button from '$lib/components/ui/Button.svelte';

	const appWindow = getCurrentWindow();

	let {
		totalSize = 0,
		fileCount = 0,
		selectedCount = 0,
		isProcessing = false,
		activeView = 'dashboard',
		onAddFile,
		onStartConversion,
		onChangeView
	}: {
		totalSize?: number;
		fileCount?: number;
		selectedCount?: number;
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
			<div class="col-span-8 flex items-center gap-6 mt-2">
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
						class="flex items-center gap-1 bg-gray-alpha-100 p-0.5 h-7.5 rounded border border-gray-alpha-100 pointer-events-auto"
					>
						<Button
							variant={activeView === 'dashboard' ? 'default' : 'titlebar-ghost'}
							size="sm"
							onclick={() => onChangeView('dashboard')}
							class="gap-2"
						>
							<LayoutList size={12} />
							<span>Dashboard</span>
						</Button>
						<Button
							variant={activeView === 'logs' ? 'default' : 'titlebar-ghost'}
							size="sm"
							onclick={() => onChangeView('logs')}
							class="gap-2"
						>
							<Terminal size={12} />
							<span>Logs</span>
						</Button>
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

			<div class="col-span-4 flex items-center gap-3 mt-2">
				{#if onAddFile}
					<Button
						onclick={onAddFile}
						variant="secondary"
						size="sm"
						class="gap-2 pointer-events-auto"
					>
						<Plus size={12} />
						Add Source
					</Button>
				{/if}

				{#if onStartConversion}
					<Button
						onclick={onStartConversion}
						disabled={isProcessing || selectedCount === 0}
						variant="primary"
						size="sm"
						class={cn('gap-2 pointer-events-auto', isProcessing && 'cursor-progress')}
					>
						{#if isProcessing}
							<span class="animate-pulse">PROCESSING...</span>
						{:else}
							<Play size={12} fill="currentColor" />
							START
						{/if}
					</Button>
				{/if}
			</div>
		</div>
	</div>

	<div class="absolute right-0 top-0 h-full flex items-center pointer-events-auto z-50">
		<Button
			variant="ghost"
			size="none"
			onclick={minimize}
			class="h-full w-12 rounded-none"
			title="Minimize"
		>
			<Minus size={16} />
		</Button>
		<Button
			variant="ghost"
			size="none"
			onclick={toggleMaximize}
			class="h-full w-12 rounded-none"
			title="Maximize"
		>
			<Square size={14} />
		</Button>
		<Button
			variant="destructive"
			size="none"
			onclick={close}
			class="h-full w-12 rounded-none"
			title="Close"
		>
			<X size={16} />
		</Button>
	</div>
</div>
