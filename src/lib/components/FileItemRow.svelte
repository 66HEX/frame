<script lang="ts">
	import { FileStatus, type FileItem } from '../types';
	import { Trash2 } from 'lucide-svelte';
	import { cn } from '$lib/utils/cn';
	import Button from '$lib/components/ui/Button.svelte';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';

	let {
		item,
		onRemove,
		onSelect,
		onToggleBatch,
		isSelected
	}: {
		item: FileItem;
		onRemove: (id: string) => void;
		onSelect: (id: string) => void;
		onToggleBatch: (id: string, isChecked: boolean) => void;
		isSelected: boolean;
	} = $props();

	function formatSize(bytes: number) {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	onclick={() => onSelect(item.id)}
	class={cn(
		'group flex cursor-pointer items-center border-b border-gray-alpha-100 px-4 py-3 transition-colors',
		isSelected ? 'bg-gray-alpha-100' : 'hover:bg-gray-alpha-100'
	)}
>
	<div class="grid flex-1 grid-cols-12 items-center gap-4">
		<div
			class="relative col-span-1 flex items-center justify-center"
			onclick={(e) => e.stopPropagation()}
		>
			<Checkbox
				checked={item.isSelectedForConversion}
				onchange={(e) => onToggleBatch(item.id, e.currentTarget.checked)}
			/>
		</div>

		<div class="col-span-4 flex items-center gap-2 overflow-hidden">
			<span class="truncate text-[13px] text-foreground [text-box:none]!">{item.name}</span>
		</div>

		<div class="col-span-3 text-right">
			<span class="text-gray-alpha-600 text-[13px]">{formatSize(item.size)}</span>
		</div>

		<div class="col-span-2 text-right">
			<span class="text-gray-alpha-600 text-[13px] uppercase">{item.originalFormat}</span>
		</div>

		<div class="col-span-2 text-right">
			{#if item.status === FileStatus.CONVERTING}
				<span class="text-[13px] text-ds-amber-800">{Math.round(item.progress)}%</span>
			{:else if item.status === FileStatus.COMPLETED}
				<span class="text-[13px] text-ds-blue-600">READY</span>
			{:else if item.status === FileStatus.QUEUED}
				<span class="text-gray-alpha-600 text-[13px]">QUEUED</span>
			{:else if item.status === FileStatus.ERROR}
				<span class="text-[13px] text-ds-red-600">ERROR</span>
			{:else}
				<span class="text-gray-alpha-600 text-[13px]">IDLE</span>
			{/if}
		</div>
	</div>

	<Button
		onclick={(e) => {
			e.stopPropagation();
			onRemove(item.id);
		}}
		variant="destructive"
		size="none"
		class="text-gray-alpha-600 ml-4 h-4 w-8 opacity-0 group-hover:opacity-100 hover:bg-transparent hover:text-ds-red-600"
	>
		<Trash2 size={14} />
	</Button>
</div>
