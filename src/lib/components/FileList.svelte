<script lang="ts">
	import FileItemRow from '$lib/components/FileItemRow.svelte';
	import type { FileItem } from '$lib/types';
	import { Check } from 'lucide-svelte';

	let {
		files,
		selectedFileId,
		onSelect,
		onRemove,
		onToggleBatch,
		onToggleAllBatch
	}: {
		files: FileItem[];
		selectedFileId: string | null;
		onSelect: (id: string) => void;
		onRemove: (id: string) => void;
		onToggleBatch: (id: string, isChecked: boolean) => void;
		onToggleAllBatch: (isChecked: boolean) => void;
	} = $props();

	let allChecked = $derived(files.length > 0 && files.every((f) => f.isSelectedForConversion));
	let isIndeterminate = $derived(
		files.some((f) => f.isSelectedForConversion) && !files.every((f) => f.isSelectedForConversion)
	);
</script>

<div
	class="col-span-12 lg:col-span-8 border border-gray-alpha-100 bg-gray-alpha-100 rounded-lg overflow-hidden flex flex-col relative group"
>
	<div class="h-10 border-b border-gray-alpha-100 flex items-center px-4 z-10">
		<div
			class="flex-1 grid grid-cols-12 gap-4 text-[10px] text-gray-alpha-600 font-medium uppercase tracking-widest items-center"
		>
			<div class="col-span-1 flex items-center justify-center relative">
				<input
					type="checkbox"
					class="appearance-none w-3.5 h-3.5 border border-gray-alpha-400 rounded-sm bg-transparent checked:bg-ds-blue-600 checked:border-ds-blue-600 transition-colors cursor-pointer"
					checked={allChecked}
					indeterminate={isIndeterminate}
					onchange={(e) => onToggleAllBatch(e.currentTarget.checked)}
				/>
				{#if allChecked}
					<Check size={10} class="absolute text-foreground pointer-events-none" />
				{/if}
			</div>
			<div class="col-span-4">Name</div>
			<div class="col-span-3 text-right">Size</div>
			<div class="col-span-2 text-right">Target</div>
			<div class="col-span-2 text-right">State</div>
		</div>
		<div class="w-8 ml-4"></div>
	</div>

	<div class="flex-1 overflow-y-auto z-10 relative">
		{#if files.length === 0}
			<div class="h-full flex flex-col items-center justify-center p-10 select-none">
				<div class="text-[10px] text-gray-alpha-600 font-medium uppercase">
					Drop files or use ADD SOURCE
				</div>
			</div>
		{:else}
			<div>
				{#each files as file (file.id)}
					<FileItemRow
						item={file}
						isSelected={selectedFileId === file.id}
						{onSelect}
						{onRemove}
						{onToggleBatch}
					/>
				{/each}
				<div class="p-4 text-center border-t border-gray-alpha-100 mt-2">
					<span class="text-[10px] text-gray-alpha-600 uppercase tracking-widest">
						END OF LIST // {files.length} OBJECTS
					</span>
				</div>
			</div>
		{/if}
	</div>
</div>
