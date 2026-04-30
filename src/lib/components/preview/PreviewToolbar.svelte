<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import {
		IconCrop as CropIcon,
		IconFileImage,
		IconFlipHorizontal as FlipHorizontalIcon,
		IconFlipVertical as FlipVerticalIcon,
		IconRotateCw
	} from '$lib/icons';
	import type { CropRect } from '$lib/utils/crop';
	import { _ } from '$lib/i18n';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	let {
		controlsDisabled,
		flipHorizontal,
		flipVertical,
		cropMode,
		appliedCrop,
		overlayMode,
		hasOverlay,
		overlayAvailable,
		hasCropDimensions,
		onRotate,
		onToggleFlip,
		onToggleCrop,
		onToggleOverlay
	}: {
		controlsDisabled: boolean;
		flipHorizontal: boolean;
		flipVertical: boolean;
		cropMode: boolean;
		appliedCrop: CropRect | null;
		overlayMode: boolean;
		hasOverlay: boolean;
		overlayAvailable: boolean;
		hasCropDimensions: boolean;
		onRotate: () => void;
		onToggleFlip: (axis: 'horizontal' | 'vertical') => void;
		onToggleCrop: () => void;
		onToggleOverlay: () => void;
	} = $props();
</script>

<div
	class="button-highlight pointer-events-auto absolute! top-1/2 left-4 z-40 flex -translate-y-1/2 flex-col gap-2 rounded-md bg-background p-1 shadow-xl"
>
	<Tooltip content={$_(`toolbar.rotate`)}>
		<Button
			size="icon"
			variant="ghost"
			onclick={(event) => {
				event.stopPropagation();
				onRotate();
			}}
			onmousedown={(event) => event.stopPropagation()}
			disabled={controlsDisabled}
		>
			<IconRotateCw size={16} />
		</Button>
	</Tooltip>
	<Tooltip content={$_(`toolbar.flip-horizontal`)}>
		<Button
			size="icon"
			variant={flipHorizontal ? 'default' : 'ghost'}
			onclick={(event) => {
				event.stopPropagation();
				onToggleFlip('horizontal');
			}}
			onmousedown={(event) => event.stopPropagation()}
			disabled={controlsDisabled}
		>
			<FlipHorizontalIcon size={16} />
		</Button>
	</Tooltip>
	<Tooltip content={$_(`toolbar.flip-vertical`)}>
		<Button
			size="icon"
			variant={flipVertical ? 'default' : 'ghost'}
			onclick={(event) => {
				event.stopPropagation();
				onToggleFlip('vertical');
			}}
			onmousedown={(event) => event.stopPropagation()}
			disabled={controlsDisabled}
		>
			<FlipVerticalIcon size={16} />
		</Button>
	</Tooltip>
	<Tooltip content={$_(`toolbar.crop`)}>
		<Button
			size="icon"
			variant={cropMode ? 'default' : appliedCrop ? 'default' : 'ghost'}
			onclick={(event) => {
				event.stopPropagation();
				onToggleCrop();
			}}
			onmousedown={(event) => event.stopPropagation()}
			disabled={controlsDisabled || !hasCropDimensions}
		>
			<CropIcon size={16} />
		</Button>
	</Tooltip>
	{#if overlayAvailable}
		<Tooltip content={$_(`toolbar.overlay`)}>
			<Button
				size="icon"
				variant={overlayMode ? 'default' : hasOverlay ? 'default' : 'ghost'}
				onclick={(event) => {
					event.stopPropagation();
					onToggleOverlay();
				}}
				onmousedown={(event) => event.stopPropagation()}
				disabled={controlsDisabled}
			>
				<IconFileImage size={16} />
			</Button>
		</Tooltip>
	{/if}
</div>
