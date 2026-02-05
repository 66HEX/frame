<script lang="ts">
	import { FileStatus, type FileItem } from '$lib/types';
	import { cn } from '$lib/utils/cn';
	import { _ } from '$lib/i18n';

	let {
		logs,
		files
	}: {
		logs: Record<string, string[]>;
		files: FileItem[];
	} = $props();

	let selectedLogFileId = $state<string | null>(null);
	let logContainer = $state<HTMLDivElement>();

	let activeFiles = $derived(files.filter((f) => logs[f.id] || f.status !== FileStatus.IDLE));

	$effect(() => {
		if (!selectedLogFileId && activeFiles.length > 0) {
			selectedLogFileId = activeFiles[0].id;
		}
	});

	let currentLogs = $derived(selectedLogFileId ? logs[selectedLogFileId] || [] : []);
	let userHasScrolledUp = $state(false);

	function handleScroll() {
		if (!logContainer) return;
		const { scrollTop, scrollHeight, clientHeight } = logContainer;
		const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
		userHasScrolledUp = !isAtBottom;
	}

	$effect(() => {
		if (currentLogs.length && logContainer && !userHasScrolledUp) {
			logContainer.scrollTop = logContainer.scrollHeight;
		}
	});

	$effect(() => {
		if (selectedLogFileId) {
			userHasScrolledUp = false;
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
					'shrink-0  text-[10px] font-medium tracking-widest uppercase transition-all',
					selectedLogFileId === file.id
						? 'text-blue-600'
						: 'text-gray-alpha-600 hover:text-foreground'
				)}
			>
				{file.name}
			</button>
		{/each}

		{#if activeFiles.length === 0}
			<span class="text-[10px] font-medium tracking-widest text-gray-alpha-600 uppercase">
				{$_('logs.noActiveProcesses')}
			</span>
		{/if}
	</div>

	<div class="relative flex flex-1 flex-col overflow-hidden">
		{#if activeFiles.length > 0}
			<div
				class="flex-1 overflow-y-auto py-4 leading-relaxed text-foreground"
				bind:this={logContainer}
				onscroll={handleScroll}
			>
				{#if currentLogs.length > 0}
					<div class="flex flex-col">
						{#each currentLogs as line, i (i)}
							<div class="group -mx-1 flex rounded px-1 py-1 text-[10px] hover:bg-gray-alpha-100">
								<span
									class="mr-3 w-8 shrink-0 pt-[0.5px] text-right text-[10px] text-gray-alpha-400 select-none"
									>{i + 1}</span
								>
								<span class="break-all whitespace-nowrap text-gray-alpha-600">{line}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div
						class="flex h-full flex-col items-center justify-center space-y-2 text-gray-alpha-600 select-none"
					>
						<div class="text-[10px] font-medium tracking-widest uppercase">
							Process started, waiting for output...
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div
				class="flex h-full flex-col items-center justify-center space-y-2 text-gray-alpha-600 select-none"
			>
				<div class="text-[10px] font-medium tracking-widest uppercase">
					{$_('logs.selectTask')}
				</div>
			</div>
		{/if}
	</div>
</div>
