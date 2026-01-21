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
	class="h-full border border-gray-alpha-100 bg-gray-alpha-100 rounded-lg overflow-hidden flex flex-col"
>
	<div class="h-10 border-b border-gray-alpha-100 flex items-center px-4 overflow-x-auto gap-6">
		{#each activeFiles as file (file.id)}
			<button
				onclick={() => (selectedLogFileId = file.id)}
				class={cn(
					'text-[10px]  uppercase tracking-widest font-medium transition-all shrink-0',
					selectedLogFileId === file.id
						? 'text-ds-blue-600'
						: 'text-gray-alpha-600 hover:text-foreground'
				)}
			>
				{file.name}
			</button>
		{/each}

		{#if activeFiles.length === 0}
			<span class="text-[10px] text-gray-alpha-600 uppercase tracking-widest font-medium">
				No active processes
			</span>
		{/if}
	</div>

	<div class="flex-1 overflow-hidden flex flex-col relative">
		{#if activeFiles.length > 0}
			<div
				class="flex-1 overflow-y-auto py-4 text-foreground leading-relaxed"
				bind:this={logContainer}
			>
				{#if currentLogs.length > 0}
					<div class="flex flex-col">
						{#each currentLogs as line, i (i)}
							<div class="flex hover:bg-gray-alpha-100 rounded px-1 -mx-1 py-1 group text-[10px]">
								<span
									class="select-none w-8 text-right mr-3 shrink-0 text-gray-alpha-600 text-[10px] pt-[0.5px]"
									>{i + 1}</span
								>
								<span class="break-all whitespace-nowrap text-gray-alpha-400">{line}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div
						class="h-full flex flex-col items-center justify-center text-gray-alpha-600 space-y-2 select-none"
					>
						<div class="text-[10px] uppercase tracking-widest font-medium">
							Process started, waiting for output...
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div
				class="h-full flex flex-col items-center justify-center text-gray-alpha-600 space-y-2 select-none"
			>
				<div class="text-[10px] uppercase tracking-widest font-medium">
					Select a task to view console output
				</div>
			</div>
		{/if}
	</div>
</div>
