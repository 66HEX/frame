<script lang="ts">
	import { untrack } from 'svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { cn } from '$lib/utils/cn';
	import CropAspectBar from './CropAspectBar.svelte';
	import PreviewToolbar from './PreviewToolbar.svelte';
	import PreviewZoomToolbar from './PreviewZoomToolbar.svelte';
	import { IconSpinner } from '$lib/icons';
	import type {
		PreviewCropController,
		PreviewPlaybackController,
		PreviewRendererController
	} from '$lib/features/preview';

	let {
		filePath,
		mediaKind,
		renderer,
		crop,
		playback,
		controlsDisabled,
		flipHorizontal,
		flipVertical
	}: {
		filePath: string;
		mediaKind: 'unknown' | 'video' | 'audio' | 'image';
		renderer: PreviewRendererController;
		crop: PreviewCropController;
		playback: PreviewPlaybackController;
		controlsDisabled: boolean;
		flipHorizontal: boolean;
		flipVertical: boolean;
	} = $props();

	let wrapperRef = $state<HTMLDivElement | undefined>();
	let canvasRef = $state<HTMLCanvasElement | undefined>();
	let audioRef = $state<HTMLAudioElement | undefined>();
	let isPanningPreview = $state(false);
	let isDraggingCrop = $state(false);
	let cropCursor = $state<string | null>(null);
	let panPointerId: number | null = null;
	let cropPointerId: number | null = null;
	let lastPanX = 0;
	let lastPanY = 0;
	let suppressPreviewClick = false;

	const isAudio = $derived(mediaKind === 'audio');
	const audioSrc = $derived(convertFileSrc(filePath));
	const canNavigatePreview = $derived(mediaKind !== 'unknown' && !isAudio);

	$effect(() => {
		const canvasElement = canvasRef;
		const wrapperElement = wrapperRef;
		untrack(() => {
			renderer.setCanvasElement(canvasElement);
			renderer.setWrapperElement(wrapperElement);
		});
	});

	$effect(() => {
		if (!isAudio) {
			const mediaElement = renderer.mediaElement;
			untrack(() => playback.setMediaElement(mediaElement));
		} else {
			const audioElement = audioRef;
			untrack(() => playback.setMediaElement(audioElement));
		}
	});

	function handlePreviewClick(event: MouseEvent) {
		if (suppressPreviewClick) {
			suppressPreviewClick = false;
			event.stopPropagation();
		}
	}

	function handlePreviewWheel(event: WheelEvent) {
		if (!canNavigatePreview) return;

		event.preventDefault();
		event.stopPropagation();
		renderer.zoomPreviewAt(event.clientX, event.clientY, event.deltaY);
	}

	function beginPreviewPan(event: PointerEvent) {
		if (crop.cropMode) {
			const cropTarget = renderer.getCropPointerTarget(event.clientX, event.clientY);
			if (cropTarget) {
				event.preventDefault();
				event.stopPropagation();
				suppressPreviewClick = true;
				isDraggingCrop = true;
				cropPointerId = event.pointerId;
				crop.beginCropDrag(cropTarget.handle, cropTarget.point);
				(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
				return;
			}
		}

		if (!canNavigatePreview || !renderer.hasPreviewTransform || event.button !== 0) return;

		event.preventDefault();
		event.stopPropagation();
		suppressPreviewClick = true;
		isPanningPreview = true;
		panPointerId = event.pointerId;
		lastPanX = event.clientX;
		lastPanY = event.clientY;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function handlePreviewPan(event: PointerEvent) {
		if (isDraggingCrop && cropPointerId === event.pointerId) {
			const point = renderer.getCropPoint(event.clientX, event.clientY);
			if (point) {
				crop.updateCropDrag(point);
			}
			return;
		}

		if (crop.cropMode && !isPanningPreview) {
			cropCursor = renderer.getCropPointerTarget(event.clientX, event.clientY)?.cursor ?? null;
		}

		if (!isPanningPreview || panPointerId !== event.pointerId) return;

		const deltaX = event.clientX - lastPanX;
		const deltaY = event.clientY - lastPanY;
		lastPanX = event.clientX;
		lastPanY = event.clientY;
		renderer.panPreviewBy(deltaX, deltaY);
	}

	function endPreviewPan(event: PointerEvent) {
		if (isDraggingCrop && cropPointerId === event.pointerId) {
			isDraggingCrop = false;
			cropPointerId = null;
			crop.endCropDrag();
			(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
			cropCursor = renderer.getCropPointerTarget(event.clientX, event.clientY)?.cursor ?? null;
			return;
		}

		if (!isPanningPreview || panPointerId !== event.pointerId) return;

		isPanningPreview = false;
		panPointerId = null;
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
	}
</script>

<div
	class="input-highlight relative flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-md bg-black"
	onclick={handlePreviewClick}
	onwheel={handlePreviewWheel}
	role="presentation"
>
	{#if mediaKind === 'unknown'}
		<IconSpinner size={24} class="animate-spin text-frame-gray-600" />
	{:else if isAudio}
		<audio bind:this={audioRef} src={audioSrc} class="hidden"></audio>
	{:else}
		<div
			class={cn(
				'absolute inset-0 overflow-hidden',
				canNavigatePreview && renderer.hasPreviewTransform && 'cursor-grab active:cursor-grabbing'
			)}
			bind:this={wrapperRef}
			style:cursor={crop.cropMode
				? (cropCursor ?? (renderer.hasPreviewTransform ? 'grab' : 'default'))
				: null}
			onpointerdown={beginPreviewPan}
			onpointermove={handlePreviewPan}
			onpointerup={endPreviewPan}
			onpointercancel={endPreviewPan}
			ondblclick={(event) => {
				if (!canNavigatePreview || !renderer.hasPreviewTransform) return;
				event.preventDefault();
				event.stopPropagation();
				suppressPreviewClick = true;
				renderer.resetPreviewTransform();
			}}
			role="presentation"
		>
			<canvas bind:this={canvasRef} class="absolute inset-0 block h-full w-full bg-black"></canvas>
		</div>
	{/if}

	{#if renderer.isLoading}
		<div
			class="pointer-events-none absolute inset-0 z-30 flex items-center justify-center bg-black/20"
		>
			<IconSpinner size={24} class="animate-spin text-frame-gray-600" />
		</div>
	{:else if renderer.error}
		<div
			class="pointer-events-none absolute inset-0 z-30 flex items-center justify-center bg-black/70 p-4"
		>
			<div class="max-w-sm text-center text-[10px] leading-5 text-frame-gray-600">
				{renderer.error}
			</div>
		</div>
	{/if}

	{#if crop.cropMode && crop.draftCrop}
		<CropAspectBar
			cropAspect={crop.cropAspect}
			hasCropDimensions={crop.hasCropDimensions}
			onSelectAspect={crop.selectAspect}
			onReset={crop.resetCropSelection}
			onApply={crop.applyCrop}
		/>
	{/if}

	{#if !isAudio}
		<PreviewToolbar
			{controlsDisabled}
			{flipHorizontal}
			{flipVertical}
			cropMode={crop.cropMode}
			appliedCrop={crop.appliedCrop}
			hasCropDimensions={crop.hasCropDimensions}
			onRotate={crop.handleRotateToggle}
			onToggleFlip={crop.toggleFlip}
			onToggleCrop={crop.toggleCropMode}
		/>
	{/if}

	<PreviewZoomToolbar
		disabled={!canNavigatePreview}
		onZoomIn={() => renderer.zoomPreviewBy(1)}
		onZoomOut={() => renderer.zoomPreviewBy(-1)}
	/>
</div>
