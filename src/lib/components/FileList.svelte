<script lang="ts">
	import FileItemRow from '$lib/components/FileItemRow.svelte';
	import type { FileItem } from '$lib/types';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';
	import { _ } from '$lib/i18n';

	let {
		files,
		selectedFileId,
		onSelect,
		onRemove,
		onToggleBatch,
		onToggleAllBatch,
		onPause,
		onResume
	}: {
		files: FileItem[];
		selectedFileId: string | null;
		onSelect: (id: string) => void;
		onRemove: (id: string) => void;
		onToggleBatch: (id: string, isChecked: boolean) => void;
		onToggleAllBatch: (isChecked: boolean) => void;
		onPause?: (id: string) => void;
		onResume?: (id: string) => void;
	} = $props();

	let allChecked = $derived(files.length > 0 && files.every((f) => f.isSelectedForConversion));
	let isIndeterminate = $derived(
		files.some((f) => f.isSelectedForConversion) && !files.every((f) => f.isSelectedForConversion)
	);
</script>

<div
	class="group relative flex h-full flex-col overflow-hidden rounded-lg border border-gray-alpha-100 bg-gray-alpha-100"
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
			<div class="col-span-5">{$_('common.name')}</div>
			<div class="col-span-2 text-right">{$_('common.size')}</div>
			<div class="col-span-2 text-right">{$_('common.target')}</div>
			<div class="col-span-2 text-right">{$_('common.state')}</div>
		</div>
		<div
			class="text-gray-alpha-600 ml-4 w-16 text-right text-[10px] font-medium tracking-widest uppercase"
		>
			{$_('common.actions')}
		</div>
	</div>

	<div class="relative z-10 flex-1 overflow-y-auto">
		{#if files.length === 0}
			<div class="flex h-full flex-col items-center justify-center p-10 select-none">
				<div class="text-gray-alpha-600 text-[10px] font-medium uppercase">
					{$_('fileList.dropFiles')}
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
						{onPause}
						{onResume}
					/>
				{/each}
			</div>
		{/if}
	</div>
</div>
