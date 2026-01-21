<script lang="ts">
	import { FileStatus, type FileItem } from '../types';
	import { Trash2, Check } from 'lucide-svelte';
	import { cn } from '$lib/utils/cn';

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
		'group flex items-center px-4 py-3 border-b border-gray-alpha-100 cursor-pointer transition-colors',
		isSelected ? 'bg-gray-alpha-100' : 'hover:bg-gray-alpha-100'
	)}
>
	<div class="flex-1 grid grid-cols-12 gap-4 items-center">
		<div
			class="col-span-1 flex items-center justify-center relative"
			onclick={(e) => e.stopPropagation()}
		>
			<input
				type="checkbox"
				class="appearance-none w-3.5 h-3.5 border border-gray-alpha-400 rounded-sm bg-transparent checked:bg-ds-blue-600 checked:border-ds-blue-600 transition-colors cursor-pointer"
				checked={item.isSelectedForConversion}
				onchange={(e) => onToggleBatch(item.id, e.currentTarget.checked)}
			/>
			{#if item.isSelectedForConversion}
				<Check size={10} class="absolute text-foreground pointer-events-none" />
			{/if}
		</div>

		<div class="col-span-4 flex items-center gap-2 overflow-hidden">
			<span class="text-[13px] text-foreground truncate [text-box:none]!">{item.name}</span>
		</div>

		<div class="col-span-3 text-right">
			<span class="text-[13px] text-gray-alpha-600">{formatSize(item.size)}</span>
		</div>

		<div class="col-span-2 text-right">
			<span class="text-[13px] text-gray-alpha-600 uppercase">{item.originalFormat}</span>
		</div>

		<div class="col-span-2 text-right">
			{#if item.status === FileStatus.CONVERTING}
				<span class="text-[13px] text-ds-amber-800">{Math.round(item.progress)}%</span>
			{:else if item.status === FileStatus.COMPLETED}
				<span class="text-[13px] text-ds-blue-600">READY</span>
			{:else if item.status === FileStatus.QUEUED}
				<span class="text-[13px] text-gray-alpha-600">QUEUED</span>
			{:else if item.status === FileStatus.ERROR}
				<span class="text-[13px] text-ds-red-600">ERROR</span>
			{:else}
				<span class="text-[13px] text-gray-alpha-600">IDLE</span>
			{/if}
		</div>
	</div>

	<button
		onclick={(e) => {
			e.stopPropagation();
			onRemove(item.id);
		}}
		class="ml-4 w-8 h-4 flex items-center justify-center opacity-0 group-hover:opacity-100 text-gray-alpha-600 hover:text-ds-red-600 transition-all"
	>
		<Trash2 size={14} />
	</button>
</div>
