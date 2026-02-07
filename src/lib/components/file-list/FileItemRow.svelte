<script lang="ts">
	import { FileStatus, type FileItem } from '$lib/types';
	import { IconTrash, IconPause, IconPlay } from '$lib/icons';
	import { cn } from '$lib/utils/cn';
	import Button from '$lib/components/ui/Button.svelte';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';
	import { _ } from '$lib/i18n';

	let {
		item,
		onRemove,
		onSelect,
		onToggleBatch,
		onPause,
		onResume,
		isSelected
	}: {
		item: FileItem;
		onRemove: (id: string) => void;
		onSelect: (id: string) => void;
		onToggleBatch: (id: string, isChecked: boolean) => void;
		onPause?: (id: string) => void;
		onResume?: (id: string) => void;
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
		'group flex h-10 cursor-pointer items-center border-b border-gray-alpha-100 px-4 transition-colors',
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

		<div class="col-span-5 flex items-center gap-2 overflow-hidden">
			<span class="truncate text-[13px] text-foreground [text-box:none]!">{item.name}</span>
		</div>

		<div class="col-span-2 text-right">
			<span class="text-[13px] text-gray-alpha-600">{formatSize(item.size)}</span>
		</div>

		<div class="col-span-2 text-right">
			<span class="text-[13px] text-gray-alpha-600 uppercase">{item.originalFormat}</span>
		</div>

		<div class="col-span-2 text-right">
			{#if item.status === FileStatus.CONVERTING || item.status === FileStatus.PAUSED}
				<span
					class={cn(
						'text-[13px]',
						item.status === FileStatus.PAUSED ? 'text-gray-alpha-600' : 'text-amber-800'
					)}>{Math.round(item.progress)}%</span
				>
			{:else if item.status === FileStatus.COMPLETED}
				<span class="text-[13px] text-blue-600">{$_('fileStatus.ready')}</span>
			{:else if item.status === FileStatus.QUEUED}
				<span class="text-[13px] text-gray-alpha-600">{$_('fileStatus.queued')}</span>
			{:else if item.status === FileStatus.ERROR}
				<span class="text-[13px] text-red-600">{$_('fileStatus.error')}</span>
			{:else}
				<span class="text-[13px] text-gray-alpha-600">{$_('fileStatus.idle')}</span>
			{/if}
		</div>
	</div>

	<div class="ml-4 flex w-16 items-center justify-end gap-2">
		<div class="flex items-center gap-2 opacity-0 transition-opacity group-hover:opacity-100">
			{#if item.status === FileStatus.CONVERTING}
				<button
					onclick={(e) => {
						e.stopPropagation();
						onPause?.(item.id);
					}}
					class="text-gray-alpha-600 transition-colors hover:text-foreground"
				>
					<IconPause size={16} fill="currentColor" color="none" />
				</button>
			{:else if item.status === FileStatus.PAUSED}
				<button
					onclick={(e) => {
						e.stopPropagation();
						onResume?.(item.id);
					}}
					class="text-gray-alpha-600 transition-colors hover:text-foreground"
				>
					<IconPlay size={16} color="currentColor" />
				</button>
			{/if}

			<Button
				onclick={(e) => {
					e.stopPropagation();
					onRemove(item.id);
				}}
				variant="destructive"
				size="none"
				disabled={item.status === FileStatus.CONVERTING}
				class="h-4 w-4 text-gray-alpha-600 hover:bg-transparent hover:text-red-600 disabled:pointer-events-none disabled:opacity-50"
			>
				<IconTrash size={16} />
			</Button>
		</div>
	</div>
</div>
