<script lang="ts">
	import { FileStatus, type FileItem } from '$lib/types';
	import { cn } from '$lib/utils/cn';

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

	$effect(() => {
		if (currentLogs.length && logContainer) {
			logContainer.scrollTop = logContainer.scrollHeight;
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
						? 'text-ds-blue-600'
						: 'text-gray-alpha-600 hover:text-foreground'
				)}
			>
				{file.name}
			</button>
		{/each}

		{#if activeFiles.length === 0}
			<span class="text-gray-alpha-600 text-[10px] font-medium tracking-widest uppercase">
				No active processes
			</span>
		{/if}
	</div>

	<div class="relative flex flex-1 flex-col overflow-hidden">
		{#if activeFiles.length > 0}
			<div
				class="flex-1 overflow-y-auto py-4 leading-relaxed text-foreground"
				bind:this={logContainer}
			>
				{#if currentLogs.length > 0}
					<div class="flex flex-col">
						{#each currentLogs as line, i (i)}
							<div class="group -mx-1 flex rounded px-1 py-1 text-[10px] hover:bg-gray-alpha-100">
								<span
									class="text-gray-alpha-600 mr-3 w-8 shrink-0 pt-[0.5px] text-right text-[10px] select-none"
									>{i + 1}</span
								>
								<span class="text-gray-alpha-400 break-all whitespace-nowrap">{line}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div
						class="text-gray-alpha-600 flex h-full flex-col items-center justify-center space-y-2 select-none"
					>
						<div class="text-[10px] font-medium tracking-widest uppercase">
							Process started, waiting for output...
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div
				class="text-gray-alpha-600 flex h-full flex-col items-center justify-center space-y-2 select-none"
			>
				<div class="text-[10px] font-medium tracking-widest uppercase">
					Select a task to view console output
				</div>
			</div>
		{/if}
	</div>
</div>
