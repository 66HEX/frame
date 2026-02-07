<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import {
		IconPlus,
		IconPlay,
		IconFileVideo,
		IconHardDrive,
		IconLayoutList,
		IconTerminal,
		IconSettings
	} from '$lib/icons';
	import { cn } from '$lib/utils/cn';
	import frameIcon from '$lib/assets/icons/frame.svg?raw';
	import Button from '$lib/components/ui/Button.svelte';
	import { _ } from '$lib/i18n';

	const appWindow = getCurrentWindow();

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
	class="z-50 flex w-full shrink-0 items-center justify-between px-4 pt-2 select-none"
	data-tauri-drag-region
>
	<div class="pointer-events-none mt-2 flex items-center gap-6">
		<div class="group pointer-events-auto z-50 mr-2 flex items-center">
			<button
				onclick={close}
				class="flex size-6 items-center justify-center rounded-full transition-opacity"
				title={$_('titlebar.close')}
			>
				<svg viewBox="-10 -10 20 20" class="h-full w-full" aria-hidden="true">
					<circle r="6" fill="#ff5f56" stroke="#e0443e" stroke-width="0.6" />
					<path
						d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8"
						stroke="#4a0002"
						stroke-width="1.5"
						stroke-linecap="round"
						class="opacity-0 transition-opacity duration-150 group-hover:opacity-100"
					/>
				</svg>
			</button>
			<button
				onclick={minimize}
				class="flex size-6 items-center justify-center rounded-full transition-opacity"
				title={$_('titlebar.minimize')}
			>
				<svg viewBox="-10 -10 20 20" class="h-full w-full" aria-hidden="true">
					<circle r="6" fill="#ffbd2e" stroke="#dea123" stroke-width="0.6" />
					<line
						x1="-2.4"
						y1="0"
						x2="2.4"
						y2="0"
						stroke="#5a3900"
						stroke-width="1.5"
						stroke-linecap="round"
						class="opacity-0 transition-opacity duration-150 group-hover:opacity-100"
					/>
				</svg>
			</button>
			<button
				onclick={toggleMaximize}
				class="flex size-6 items-center justify-center rounded-full transition-opacity"
				title={$_('titlebar.toggleSize')}
			>
				<svg viewBox="-10 -10 20 20" class="h-full w-full" aria-hidden="true">
					<circle r="6" fill="#27c93f" stroke="#1aab29" stroke-width="0.6" />
					<g
						fill="#004200"
						class="opacity-0 transition-opacity duration-150 group-hover:opacity-100"
					>
						<path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z" />
						<path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z" />
					</g>
				</svg>
			</button>
		</div>

		<span
			class="pointer-events-none flex items-center justify-center px-2 text-foreground [&>svg]:size-5 [&>svg]:fill-current [&>svg]:opacity-60"
			aria-hidden="true"
		>
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html frameIcon}
		</span>

		<div class="pointer-events-none h-6 w-px bg-gray-alpha-100"></div>

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
					<IconLayoutList size={14} />
					<span>{$_('titlebar.dashboard')}</span>
				</Button>
				<Button
					variant={activeView === 'logs' ? 'default' : 'titlebar-ghost'}
					size="sm"
					onclick={() => onChangeView('logs')}
					class="gap-2"
				>
					<IconTerminal size={14} />
					<span>{$_('titlebar.logs')}</span>
				</Button>
			</div>
		{/if}

		<div class="pointer-events-none h-6 w-px bg-gray-alpha-100"></div>

		<div class="pointer-events-none flex items-center gap-4 text-[10px] text-gray-alpha-600">
			<div class="flex items-center gap-2">
				<IconHardDrive size={14} />
				<span>{$_('titlebar.storage')} {formatTotalSize(totalSize)}</span>
			</div>
			<div class="flex items-center gap-2">
				<IconFileVideo size={14} />
				<span>{$_('titlebar.items')} {fileCount}</span>
			</div>
		</div>
	</div>

	<div class="pointer-events-none mt-2 flex items-center gap-3">
		{#if onOpenSettings}
			<Button onclick={onOpenSettings} variant="secondary" size="icon" class="pointer-events-auto">
				<IconSettings size={16} />
			</Button>
		{/if}
		{#if onAddFile}
			<Button onclick={onAddFile} variant="secondary" class="pointer-events-auto gap-2">
				<IconPlus size={14} />
				{$_('titlebar.addSource')}
			</Button>
		{/if}

		{#if onStartConversion}
			<Button
				onclick={onStartConversion}
				disabled={isProcessing || selectedCount === 0}
				variant="default"
				class={cn('pointer-events-auto gap-2', isProcessing && 'cursor-progress')}
			>
				{#if isProcessing}
					<span class="animate-pulse">{$_('titlebar.processing')}</span>
				{:else}
					<IconPlay size={14} color="currentColor" />
					{$_('titlebar.start')}
				{/if}
			</Button>
		{/if}
	</div>
</div>
