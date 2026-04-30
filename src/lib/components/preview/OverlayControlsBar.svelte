<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import Slider from '$lib/components/ui/Slider.svelte';
	import { IconCheck, IconFileImage, IconMinus, IconPlus, IconTrash } from '$lib/icons';
	import type { OverlaySettings } from '$lib/types';
	import { _ } from '$lib/i18n';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	let {
		overlay,
		onReplace,
		onOpacity,
		onSize,
		onRemove,
		onDone
	}: {
		overlay: OverlaySettings;
		onReplace: () => void;
		onOpacity: (value: number) => void;
		onSize: (direction: 1 | -1) => void;
		onRemove: () => void;
		onDone: () => void;
	} = $props();
</script>

<div
	class="button-highlight pointer-events-auto absolute! bottom-4 left-1/2 z-50 flex max-w-[calc(100%-2rem)] -translate-x-1/2 items-center gap-2 rounded-md bg-background p-1 text-[10px] shadow-xl"
	onclick={(event) => event.stopPropagation()}
	onpointerdown={(event) => event.stopPropagation()}
	role="presentation"
>
	<Tooltip content={$_(`preview.replace`)}>
		<Button size="icon" variant="ghost" onclick={onReplace}>
			<IconFileImage size={16} />
		</Button>
	</Tooltip>

	<div class="h-4 w-px bg-frame-gray-200"></div>

	<Tooltip content={$_(`preview.decrease`)}>
		<Button size="icon" variant="ghost" onclick={() => onSize(-1)}>
			<IconMinus size={16} />
		</Button>
	</Tooltip>
	<Tooltip content={$_(`preview.increase`)}>
		<Button size="icon" variant="ghost" onclick={() => onSize(1)}>
			<IconPlus size={16} />
		</Button>
	</Tooltip>

	<div class="flex w-28 items-center px-1">
		<Tooltip class="flex h-7.5 w-full items-center" content={$_(`preview.opacity`)}>
			<Slider
				value={Math.round(overlay.opacity * 100)}
				min={0}
				max={100}
				step={1}
				oninput={(event) =>
					onOpacity(Number((event.currentTarget as HTMLInputElement).value) / 100)}
			/>
		</Tooltip>
	</div>

	<div class="h-4 w-px bg-frame-gray-200"></div>

	<Tooltip content={$_(`preview.remove`)}>
		<Button size="icon" variant="ghost-destructive" onclick={onRemove}>
			<IconTrash size={16} />
		</Button>
	</Tooltip>
	<Tooltip content={$_(`common.done`)}>
		<Button size="icon" onclick={onDone}>
			<IconCheck size={16} />
		</Button>
	</Tooltip>
</div>
