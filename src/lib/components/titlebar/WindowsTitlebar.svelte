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

<div class="relative z-50 h-10 w-full shrink-0 select-none" data-tauri-drag-region>
	<div class="pointer-events-none absolute inset-0 flex items-center px-4">
		<div class="grid w-full grid-cols-12 gap-4">
			<div class="col-span-8 mt-2 flex items-center gap-6">
				<span
					class="pointer-events-none flex items-center justify-center text-foreground [&>svg]:size-5 [&>svg]:fill-current [&>svg]:opacity-60"
					aria-hidden="true"
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html frameIcon}
				</span>

				<div class="pointer-events-none h-5 w-px bg-gray-alpha-100"></div>

				{#if onChangeView}
					<div
						class="pointer-events-auto flex h-7.5 items-center gap-1 rounded border border-gray-alpha-100 bg-gray-alpha-100 p-0.5"
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

				<div class="pointer-events-none h-5 w-px bg-gray-alpha-100"></div>

				<div class="text-gray-alpha-600 pointer-events-none flex items-center gap-4 text-[10px]">
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

			<div class="col-span-4 mt-2 flex items-center gap-3">
				{#if onAddFile}
					<Button
						onclick={onAddFile}
						variant="secondary"
						size="sm"
						class="pointer-events-auto gap-2"
					>
						<Plus size={12} />
						Add Source
					</Button>
				{/if}

				{#if onStartConversion}
					<Button
						onclick={onStartConversion}
						disabled={isProcessing || selectedCount === 0}
						variant="default"
						size="sm"
						class={cn('pointer-events-auto gap-2', isProcessing && 'cursor-progress')}
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

	<div class="pointer-events-auto absolute top-0 right-0 z-50 flex h-full items-center">
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
			variant="titlebar-destructive"
			size="none"
			onclick={close}
			class="h-full w-12 rounded-none"
			title="Close"
		>
			<X size={16} />
		</Button>
	</div>
</div>
