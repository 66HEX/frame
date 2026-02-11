<script lang="ts">
	import { FileStatus, type FileItem } from '$lib/types';
	import { cn } from '$lib/utils/cn';
	import { _ } from '$lib/i18n';
	import VirtualList from '@humanspeak/svelte-virtual-list';
	import type { SvelteVirtualListScrollOptions } from '@humanspeak/svelte-virtual-list';
	import { tick, untrack } from 'svelte';
	import { IconArrowDown } from '$lib/icons';
	import { getHighlighter, highlightLogLineSync } from '$lib/services/shiki';
	import type { HighlighterCore } from 'shiki/core';

	let {
		logs,
		files
	}: {
		logs: Record<string, string[]>;
		files: FileItem[];
	} = $props();

	let selectedLogFileId = $state<string | null>(null);
	let virtualListRef = $state<{
		scroll: (options: SvelteVirtualListScrollOptions) => void;
	} | null>(null);

	let highlighter = $state<HighlighterCore | null>(null);

	$effect(() => {
		getHighlighter().then((hl) => {
			highlighter = hl;
		});
	});

	let activeFiles = $derived(files.filter((f) => logs[f.id] || f.status !== FileStatus.IDLE));

	$effect(() => {
		if (!selectedLogFileId && activeFiles.length > 0) {
			selectedLogFileId = activeFiles[0].id;
		}
	});

	let currentLogs = $derived(selectedLogFileId ? logs[selectedLogFileId] || [] : []);
	let logsWithIndex = $derived(currentLogs.map((line, i) => ({ line, index: i + 1 })));

	let shouldStickToBottom = $state(true);
	let wrapperDiv = $state<HTMLDivElement>();

	function handleScroll(e: Event) {
		const target = e.target as HTMLDivElement;
		const { scrollTop, scrollHeight, clientHeight } = target;

		const isAtBottom = scrollHeight - scrollTop - clientHeight < 25;
		shouldStickToBottom = isAtBottom;
	}

	function scrollToBottom() {
		if (virtualListRef && logsWithIndex.length > 0) {
			shouldStickToBottom = true;
			virtualListRef.scroll({ index: logsWithIndex.length - 1, align: 'bottom' });
		}
	}

	$effect(() => {
		if (virtualListRef && wrapperDiv) {
			const viewport = wrapperDiv.querySelector('.logs-viewport');
			if (viewport) {
				viewport.addEventListener('scroll', handleScroll);
				return () => viewport.removeEventListener('scroll', handleScroll);
			}
		}
	});

	$effect(() => {
		const length = logsWithIndex.length;
		if (length > 0 && virtualListRef) {
			const stick = untrack(() => shouldStickToBottom);
			if (stick || length === 1) {
				tick().then(() => {
					if (untrack(() => shouldStickToBottom) && virtualListRef) {
						virtualListRef.scroll({
							index: length - 1,
							align: 'bottom',
							smoothScroll: false
						});
					}
				});
			}
		}
	});

	$effect(() => {
		const id = selectedLogFileId;
		if (id) {
			untrack(() => {
				shouldStickToBottom = true;
				if (virtualListRef && logsWithIndex.length > 0) {
					tick().then(() => {
						virtualListRef?.scroll({
							index: logsWithIndex.length - 1,
							align: 'bottom',
							smoothScroll: false
						});
					});
				}
			});
		}
	});
</script>

<div
	class="flex h-full flex-col overflow-hidden rounded-lg border border-gray-alpha-100 bg-gray-alpha-100"
>
	<div class="flex h-10 items-center gap-6 overflow-x-auto border-b border-gray-alpha-100 px-4">
		{#each activeFiles as file (file.id)}
			<button
				onclick={() => (selectedLogFileId = file.id)}
				class={cn(
					'shrink-0 text-[10px] font-medium tracking-widest transition-all',
					selectedLogFileId === file.id
						? 'text-blue-600'
						: 'text-gray-alpha-600 hover:text-foreground'
				)}
			>
				{file.name}
			</button>
		{/each}

		{#if activeFiles.length === 0}
			<span class="text-[10px] font-medium tracking-widest text-gray-alpha-600">
				{$_('logs.noActiveProcesses')}
			</span>
		{/if}
	</div>

	<div class="relative flex flex-1 flex-col overflow-hidden" bind:this={wrapperDiv}>
		{#if activeFiles.length > 0}
			<div class="relative h-full flex-1 p-0.5 leading-relaxed text-foreground">
				{#if logsWithIndex.length > 0}
					<VirtualList
						items={logsWithIndex}
						bind:this={virtualListRef}
						viewportClass="logs-viewport"
						defaultEstimatedItemHeight={24}
					>
						{#snippet renderItem(item)}
							<div class="group -mx-1 flex rounded px-1 py-0.5 text-[10px] hover:bg-gray-alpha-100">
								<span
									class="mr-3 w-8 shrink-0 pt-[0.5px] text-right text-[10px] text-gray-alpha-400 select-none"
									>{item.index}</span
								>
								<span class="log-line break-all whitespace-nowrap">
									{#if highlighter}
										<!-- eslint-disable-next-line svelte/no-at-html-tags -->
										{@html highlightLogLineSync(highlighter, item.line)}
									{:else}
										{item.line}
									{/if}
								</span>
							</div>
						{/snippet}
					</VirtualList>
				{:else}
					<div
						class="flex h-full flex-col items-center justify-center space-y-2 text-gray-alpha-600 select-none"
					>
						<div class="text-[10px] font-medium tracking-widest">
							{$_('logs.waitingForOutput')}
						</div>
					</div>
				{/if}

				{#if !shouldStickToBottom}
					<button
						onclick={scrollToBottom}
						class="absolute right-4 bottom-4 z-10 rounded-full bg-blue-600 p-2 text-foreground shadow-lg backdrop-blur-md transition-all"
						title={$_('logs.scrollToBottom')}
					>
						<IconArrowDown size={14} />
					</button>
				{/if}
			</div>
		{:else}
			<div
				class="flex h-full flex-col items-center justify-center space-y-2 text-gray-alpha-600 select-none"
			>
				<div class="text-[10px] font-medium tracking-widest">
					{$_('logs.selectTask')}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	:global(.logs-viewport) {
		height: 100% !important;
		overflow-y: auto !important;
	}

	.log-line :global(pre),
	.log-line :global(code) {
		display: inline;
		background: transparent !important;
		padding: 0 !important;
		margin: 0 !important;
		font-size: inherit;
		/* font-family: inherit; */
		line-height: inherit;
	}

	.log-line :global(pre.shiki) {
		background: transparent !important;
	}
</style>
