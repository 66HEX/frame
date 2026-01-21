<script lang="ts">
	import FileItemRow from '$lib/components/FileItemRow.svelte';
	import type { FileItem } from '$lib/types';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';

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
	class="group relative col-span-12 flex flex-col overflow-hidden rounded-lg border border-gray-alpha-100 bg-gray-alpha-100 lg:col-span-8"
>
	<div class="z-10 flex h-10 items-center border-b border-gray-alpha-100 px-4">
		<div
			class="text-gray-alpha-600 grid flex-1 grid-cols-12 items-center gap-4 text-[10px] font-medium tracking-widest uppercase"
		>
			<div class="relative col-span-1 flex items-center justify-center">
				<Checkbox
					checked={allChecked}
					indeterminate={isIndeterminate}
					onchange={(e) => onToggleAllBatch(e.currentTarget.checked)}
				/>
			</div>
			<div class="col-span-4">Name</div>
			<div class="col-span-3 text-right">Size</div>
			<div class="col-span-2 text-right">Target</div>
			<div class="col-span-2 text-right">State</div>
		</div>
		<div class="ml-4 w-8"></div>
	</div>

	<div class="relative z-10 flex-1 overflow-y-auto">
		{#if files.length === 0}
			<div class="flex h-full flex-col items-center justify-center p-10 select-none">
				<div class="text-gray-alpha-600 text-[10px] font-medium uppercase">
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
				<div class="mt-2 border-t border-gray-alpha-100 p-4 text-center">
					<span class="text-gray-alpha-600 text-[10px] tracking-widest uppercase">
						END OF LIST // {files.length} OBJECTS
					</span>
				</div>
			</div>
		{/if}
	</div>
</div>
