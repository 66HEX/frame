<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import {
		IconCrop as CropIcon,
		IconFlipHorizontal as FlipHorizontalIcon,
		IconFlipVertical as FlipVerticalIcon,
		IconPlay,
		IconPause2,
		IconRotateCw
	} from '$lib/icons';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import TimecodeInput from '$lib/components/ui/TimecodeInput.svelte';
	import { _ } from '$lib/i18n';
	import type { ConversionConfig, CropSettings } from '$lib/types';
	import {
		type CropRect,
		type DragHandle,
		MIN_CROP,
		ASPECT_OPTIONS,
		getAspectValue,
		clamp,
		clampRect,
		transformCropRect,
		remapDragDeltas,
		adjustRectToRatio,
		enforceAspect,
		getHandleCursor
	} from '$lib/utils/crop';

	const ROTATION_STEPS: ConversionConfig['rotation'][] = ['0', '90', '180', '270'];

	let {
		filePath,

		initialStartTime,
		initialEndTime,
		rotation = '0',
		flipHorizontal = false,
		flipVertical = false,
		onSave,
		onUpdateConfig,
		controlsDisabled = false,
		initialCrop = null,
		sourceWidth,
		sourceHeight
	}: {
		filePath: string;
		initialStartTime?: string;
		initialEndTime?: string;
		rotation?: ConversionConfig['rotation'];
		flipHorizontal?: boolean;
		flipVertical?: boolean;
		onSave: (start?: string, end?: string) => void;
		onUpdateConfig?: (config: Partial<ConversionConfig>) => void;
		controlsDisabled?: boolean;
		initialCrop?: CropSettings | null;
		sourceWidth?: number;
		sourceHeight?: number;
	} = $props();

	let videoSrc = $state('');
	let containerRef: HTMLDivElement | undefined = $state();
	let videoWrapperRef: HTMLDivElement | undefined = $state();
	let videoRef: HTMLVideoElement | undefined = $state();
	let containerWidth = $state(0);
	let containerHeight = $state(0);
	let videoBounds = $state({ width: 0, height: 0 });
	let naturalVideoWidth = $state(0);
	let naturalVideoHeight = $state(0);

	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let startValue = $state(0);
	let endValue = $state(0);
	let previousInitialStart: string | undefined;
	let previousInitialEnd: string | undefined;

	let sliderRef: HTMLDivElement | undefined = $state();
	let dragging: 'start' | 'end' | 'scrub' | null = null;
	let wasPlayingBeforeScrub = false;
	let isHovering = $state(false);

	let cropMode = $state(false);
	let appliedCrop: CropRect | null = $state(null);
	let draftCrop: CropRect | null = $state(null);
	let cropAspect = $state<string>('free');
	let cropHandle: DragHandle | null = null;
	let cropDragOrigin: {
		handle: DragHandle;
		startRect: CropRect;
		startX: number;
		startY: number;
	} | null = null;

	const isSideRotation = $derived(rotation === '90' || rotation === '270');

	const videoStyle = $derived.by(() => {
		if (!containerWidth || !containerHeight) {
			return 'width: 100%; height: 100%;';
		}

		const baseW = sourceWidth ?? naturalVideoWidth;
		const baseH = sourceHeight ?? naturalVideoHeight;

		if (!baseW || !baseH) {
			return 'width: 100%; height: auto;';
		}

		let targetRect = { x: 0, y: 0, width: 1, height: 1 };

		if (!cropMode && appliedCrop) {
			targetRect = appliedCrop;
		}

		const contentW = baseW * targetRect.width;
		const contentH = baseH * targetRect.height;

		const visualW = isSideRotation ? contentH : contentW;
		const visualH = isSideRotation ? contentW : contentH;

		const scale = Math.min(containerWidth / visualW, containerHeight / visualH);

		const finalW = visualW * scale;
		const finalH = visualH * scale;

		return `width: ${finalW}px; height: ${finalH}px; --video-scale: ${scale};`;
	});

	const transformStyle = $derived.by(() => {
		const baseW = sourceWidth ?? naturalVideoWidth;
		const baseH = sourceHeight ?? naturalVideoHeight;

		if (!baseW || !baseH) return '';

		let crop = { x: 0, y: 0, width: 1, height: 1 };
		if (!cropMode && appliedCrop) {
			crop = appliedCrop;
		}

		const cx = crop.x + crop.width / 2 - 0.5;
		const cy = crop.y + crop.height / 2 - 0.5;
		const tx = -cx * baseW;
		const ty = -cy * baseH;

		const transforms = [
			`scale(var(--video-scale))`,
			`rotate(${rotation}deg)`,
			flipHorizontal ? 'scaleX(-1)' : '',
			flipVertical ? 'scaleY(-1)' : '',
			`translate(${tx}px, ${ty}px)`
		]
			.filter(Boolean)
			.join(' ');

		return `width: ${baseW}px; height: ${baseH}px; position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%) ${transforms};`;
	});

	const hasCropDimensions = $derived(() => {
		const baseWidth = sourceWidth ?? naturalVideoWidth;
		const baseHeight = sourceHeight ?? naturalVideoHeight;
		if (!baseWidth || !baseHeight) return false;
		if (rotation === '90' || rotation === '270') {
			return Boolean(baseHeight && baseWidth);
		}
		return true;
	});

	$effect(() => {
		if (initialStartTime !== previousInitialStart) {
			previousInitialStart = initialStartTime;
			startValue = initialStartTime ? parseTimeToSeconds(initialStartTime) : 0;
		}
	});

	$effect(() => {
		if (initialEndTime !== previousInitialEnd) {
			previousInitialEnd = initialEndTime;
			if (initialEndTime) {
				endValue = parseTimeToSeconds(initialEndTime);
			} else if (duration) {
				endValue = duration;
			}
		}
	});

	$effect(() => {
		if (initialCrop?.enabled && initialCrop.sourceWidth && initialCrop.sourceHeight) {
			const rawRect = {
				x: initialCrop.x / initialCrop.sourceWidth,
				y: initialCrop.y / initialCrop.sourceHeight,
				width: initialCrop.width / initialCrop.sourceWidth,
				height: initialCrop.height / initialCrop.sourceHeight
			};

			appliedCrop = clampRect(
				transformCropRect(rawRect, rotation, flipHorizontal, flipVertical, true)
			);

			if (initialCrop.aspectRatio) {
				cropAspect = initialCrop.aspectRatio;
			}
		} else if (!cropMode) {
			appliedCrop = null;
			if (!cropMode) {
				cropAspect = 'free';
			}
		}

		if (!cropMode) {
			draftCrop = null;
		}
	});

	onMount(() => {
		videoSrc = convertFileSrc(filePath);

		const resizeObserver = new ResizeObserver((entries) => {
			for (const entry of entries) {
				containerWidth = entry.contentRect.width;
				containerHeight = entry.contentRect.height;
			}
		});

		if (containerRef) {
			resizeObserver.observe(containerRef);
		}

		const wrapperObserver = new ResizeObserver(() => {
			updateVideoBounds();
		});

		if (videoWrapperRef) {
			wrapperObserver.observe(videoWrapperRef);
		}

		const handleResize = () => updateVideoBounds();
		window.addEventListener('resize', handleResize);

		return () => {
			resizeObserver.disconnect();
			wrapperObserver.disconnect();
			window.removeEventListener('resize', handleResize);
			detachCropListeners();
		};
	});

	function parseTimeToSeconds(timeStr?: string): number {
		if (!timeStr) return 0;
		const parts = timeStr.split(':').map(Number);
		if (parts.length === 3) {
			return parts[0] * 3600 + parts[1] * 60 + parts[2];
		}
		return 0;
	}

	function formatTime(seconds: number): string {
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = seconds % 60;
		return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toFixed(3).padStart(6, '0')}`;
	}

	function handleMetadata() {
		if (videoRef) {
			duration = videoRef.duration;
			naturalVideoWidth = videoRef.videoWidth;
			naturalVideoHeight = videoRef.videoHeight;
			updateVideoBounds();
			if (initialEndTime) {
				endValue = parseTimeToSeconds(initialEndTime);
			} else {
				endValue = duration;
			}

			if (startValue > duration) startValue = 0;
			if (endValue > duration) endValue = duration;
		}
	}

	function handleTimeUpdate() {
		if (videoRef) {
			currentTime = videoRef.currentTime;
			if (dragging) return;

			if (currentTime >= endValue) {
				videoRef.pause();
				isPlaying = false;
				videoRef.currentTime = startValue;
			}
		}
	}

	function togglePlay() {
		if (videoRef) {
			if (isPlaying) {
				videoRef.pause();
			} else {
				if (videoRef.currentTime < startValue || videoRef.currentTime >= endValue) {
					videoRef.currentTime = startValue;
				}
				videoRef.play();
			}
			isPlaying = !isPlaying;
		}
	}

	function commitTrimValues() {
		const startStr = startValue > 0 ? formatTime(startValue) : undefined;
		const endStr = duration > 0 && endValue < duration ? formatTime(endValue) : undefined;
		onSave(startStr, endStr);
	}

	function handleMouseDown(e: MouseEvent, type: 'start' | 'end') {
		e.preventDefault();
		e.stopPropagation();
		dragging = type;
		window.addEventListener('mousemove', handleMouseMove);
		window.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(e: MouseEvent) {
		if (!dragging || !sliderRef) return;
		const rect = sliderRef.getBoundingClientRect();
		const percent = Math.min(Math.max((e.clientX - rect.left) / rect.width, 0), 1);
		const time = percent * duration;

		if (dragging === 'scrub') {
			currentTime = time;
			if (videoRef) videoRef.currentTime = currentTime;
			return;
		}

		if (dragging === 'start') {
			startValue = Math.min(time, endValue - 1);
			if (videoRef) videoRef.currentTime = startValue;
		} else if (dragging === 'end') {
			endValue = Math.max(time, startValue + 1);
			if (videoRef) videoRef.currentTime = endValue;
		}
		commitTrimValues();
	}

	function handleMouseUp() {
		if (dragging === 'scrub') {
			if (wasPlayingBeforeScrub && videoRef) {
				videoRef.play();
			}
		} else if (dragging) {
			commitTrimValues();
		}
		dragging = null;
		window.removeEventListener('mousemove', handleMouseMove);
		window.removeEventListener('mouseup', handleMouseUp);
	}

	function seekTo(e: MouseEvent) {
		if (!sliderRef) return;
		const rect = sliderRef.getBoundingClientRect();
		const percent = Math.min(Math.max((e.clientX - rect.left) / rect.width, 0), 1);
		const time = percent * duration;
		if (videoRef) {
			videoRef.currentTime = time;
			currentTime = time;
		}

		dragging = 'scrub';
		wasPlayingBeforeScrub = isPlaying;
		if (isPlaying && videoRef) {
			videoRef.pause();
		}
		window.addEventListener('mousemove', handleMouseMove);
		window.addEventListener('mouseup', handleMouseUp);
	}

	function updateVideoBounds() {
		if (!videoWrapperRef) return;
		const rect = videoWrapperRef.getBoundingClientRect();
		videoBounds = { width: rect.width, height: rect.height };
	}

	function defaultCropRect(): CropRect {
		return { x: 0.1, y: 0.1, width: 0.8, height: 0.8 };
	}

	function toggleCropMode() {
		if (controlsDisabled || !hasCropDimensions) return;
		if (cropMode) {
			cropMode = false;
			draftCrop = null;
			return;
		}
		cropMode = true;
		draftCrop = appliedCrop ? { ...appliedCrop } : defaultCropRect();
	}

	function selectAspect(id: string) {
		cropAspect = id;
		if (!draftCrop) return;
		if (id === 'free') {
			draftCrop = clampRect({ ...draftCrop });
			return;
		}
		const ratio = getAspectValue(id);
		if (!ratio) return;
		const w = sourceWidth ?? naturalVideoWidth;
		const h = sourceHeight ?? naturalVideoHeight;
		draftCrop = clampRect(adjustRectToRatio(draftCrop, ratio, w, h, isSideRotation));
	}

	function applyCrop() {
		if (!draftCrop || !hasCropDimensions) return;
		const isFull =
			draftCrop.x <= 0.001 &&
			draftCrop.y <= 0.001 &&
			draftCrop.width >= 0.999 &&
			draftCrop.height >= 0.999;
		if (isFull) {
			persistCrop(null);
			appliedCrop = null;
			cropAspect = 'free';
		} else {
			persistCrop(draftCrop);
			appliedCrop = { ...draftCrop };
		}
		cropMode = false;
		draftCrop = null;
	}

	function resetCropSelection() {
		if (!draftCrop) {
			draftCrop = defaultCropRect();
		} else {
			draftCrop = { x: 0, y: 0, width: 1, height: 1 };
		}
		cropAspect = 'free';
	}

	function persistCrop(rect: CropRect | null, overrides: Partial<ConversionConfig> = {}) {
		if (!onUpdateConfig) return;

		const nextRotation = overrides.rotation ?? rotation;
		const nextFlipH = overrides.flipHorizontal ?? flipHorizontal;
		const nextFlipV = overrides.flipVertical ?? flipVertical;

		if (!rect) {
			onUpdateConfig({ crop: null, ...overrides });
			return;
		}
		const dims = getBaseDimensions(nextRotation);
		if (!dims) return;

		const outputRect = transformCropRect(rect, nextRotation, nextFlipH, nextFlipV, false);

		const payload: CropSettings = {
			enabled: true,
			x: Math.round(outputRect.x * dims.width),
			y: Math.round(outputRect.y * dims.height),
			width: Math.round(outputRect.width * dims.width),
			height: Math.round(outputRect.height * dims.height),
			sourceWidth: dims.width,
			sourceHeight: dims.height,
			aspectRatio: cropAspect === 'free' ? null : cropAspect
		};
		onUpdateConfig({ crop: payload, ...overrides });
	}

	function handleRotateToggle() {
		if (!onUpdateConfig || controlsDisabled) return;
		const idx = ROTATION_STEPS.indexOf(rotation);
		const next = ROTATION_STEPS[(idx + 1) % ROTATION_STEPS.length];

		if (appliedCrop) {
			persistCrop(appliedCrop, { rotation: next });
		} else {
			onUpdateConfig({ rotation: next });
		}
	}

	function toggleFlip(axis: 'horizontal' | 'vertical') {
		if (!onUpdateConfig || controlsDisabled) return;

		const nextFlipH = axis === 'horizontal' ? !flipHorizontal : flipHorizontal;
		const nextFlipV = axis === 'vertical' ? !flipVertical : flipVertical;

		if (appliedCrop) {
			persistCrop(appliedCrop, { flipHorizontal: nextFlipH, flipVertical: nextFlipV });
		} else {
			onUpdateConfig({ flipHorizontal: nextFlipH, flipVertical: nextFlipV });
		}
	}

	function getBaseDimensions(rot: ConversionConfig['rotation'] = rotation) {
		let baseWidth = sourceWidth ?? naturalVideoWidth;
		let baseHeight = sourceHeight ?? naturalVideoHeight;
		if (!baseWidth || !baseHeight) return null;
		if (rot === '90' || rot === '270') {
			[baseWidth, baseHeight] = [baseHeight, baseWidth];
		}
		return { width: baseWidth, height: baseHeight };
	}

	function beginCropDrag(handle: DragHandle, event: MouseEvent) {
		if (!draftCrop || !cropMode) return;
		event.preventDefault();
		event.stopPropagation();
		cropHandle = handle;
		cropDragOrigin = {
			handle,
			startRect: { ...draftCrop },
			startX: event.clientX,
			startY: event.clientY
		};
		window.addEventListener('mousemove', handleCropDrag);
		window.addEventListener('mouseup', endCropDrag);
	}

	function handleCropDrag(event: MouseEvent) {
		if (!cropHandle || !cropDragOrigin || !draftCrop || !videoBounds.width || !videoBounds.height)
			return;
		const normalizedDx = (event.clientX - cropDragOrigin.startX) / videoBounds.width;
		const normalizedDy = (event.clientY - cropDragOrigin.startY) / videoBounds.height;

		const { dx, dy } = remapDragDeltas(
			normalizedDx,
			normalizedDy,
			rotation,
			flipHorizontal,
			flipVertical
		);
		let { startRect } = cropDragOrigin;

		if (cropHandle === 'move') {
			const nextX = clamp(startRect.x + dx, 0, 1 - startRect.width);
			const nextY = clamp(startRect.y + dy, 0, 1 - startRect.height);
			draftCrop = { x: nextX, y: nextY, width: startRect.width, height: startRect.height };
			return;
		}

		let edges = {
			left: startRect.x,
			right: startRect.x + startRect.width,
			top: startRect.y,
			bottom: startRect.y + startRect.height
		};

		if (cropHandle.includes('w')) {
			edges.left = clamp(startRect.x + dx, 0, edges.right - MIN_CROP);
		}
		if (cropHandle.includes('e')) {
			edges.right = clamp(startRect.x + startRect.width + dx, edges.left + MIN_CROP, 1);
		}
		if (cropHandle.includes('n')) {
			edges.top = clamp(startRect.y + dy, 0, edges.bottom - MIN_CROP);
		}
		if (cropHandle.includes('s')) {
			edges.bottom = clamp(startRect.y + startRect.height + dy, edges.top + MIN_CROP, 1);
		}

		let nextRect: CropRect = {
			x: edges.left,
			y: edges.top,
			width: edges.right - edges.left,
			height: edges.bottom - edges.top
		};

		if (cropAspect !== 'free') {
			const ratio = getAspectValue(cropAspect);
			if (ratio) {
				const w = sourceWidth ?? naturalVideoWidth;
				const h = sourceHeight ?? naturalVideoHeight;
				nextRect = enforceAspect(nextRect, cropHandle, startRect, ratio, w, h, isSideRotation);
			}
		}

		draftCrop = clampRect(nextRect);
	}

	function endCropDrag() {
		detachCropListeners();
		cropHandle = null;
		cropDragOrigin = null;
	}

	function detachCropListeners() {
		window.removeEventListener('mousemove', handleCropDrag);
		window.removeEventListener('mouseup', endCropDrag);
	}
</script>

<div
	class="flex h-full flex-col overflow-hidden rounded-xl border border-gray-alpha-100 bg-gray-alpha-100 p-4"
>
	<div
		class="relative flex min-h-0 flex-1 cursor-pointer items-center justify-center overflow-hidden rounded-lg border border-gray-alpha-200 bg-black"
		bind:this={containerRef}
		onclick={() => !cropMode && togglePlay()}
		onmouseenter={() => (isHovering = true)}
		onmouseleave={() => (isHovering = false)}
		role="presentation"
	>
		<div
			class="relative inline-flex max-h-full max-w-full overflow-hidden"
			bind:this={videoWrapperRef}
			style={videoStyle}
		>
			<div class="origin-center" style={transformStyle}>
				<video
					bind:this={videoRef}
					src={videoSrc}
					class="block h-full w-full bg-black object-contain"
					onloadedmetadata={handleMetadata}
					ontimeupdate={handleTimeUpdate}
					onplay={() => (isPlaying = true)}
					onpause={() => (isPlaying = false)}
				>
					<track kind="captions" />
				</video>

				{#if cropMode && draftCrop}
					<div class="pointer-events-none absolute inset-0">
						<div
							class="absolute top-0 left-0 w-full bg-black/40"
							style={`height: ${draftCrop.y * 100}%;`}
						></div>
						<div
							class="absolute left-0 bg-black/40"
							style={`top: ${draftCrop.y * 100}%; height: ${draftCrop.height * 100}%; width: ${
								draftCrop.x * 100
							}%;`}
						></div>
						<div
							class="absolute right-0 bg-black/40"
							style={`top: ${draftCrop.y * 100}%; height: ${draftCrop.height * 100}%; width: ${
								(1 - draftCrop.x - draftCrop.width) * 100
							}%;`}
						></div>
						<div
							class="absolute bottom-0 left-0 w-full bg-black/40"
							style={`height: ${(1 - draftCrop.y - draftCrop.height) * 100}%;`}
						></div>
					</div>

					<div
						class="absolute rounded border border-foreground shadow-xl"
						style={`left: ${draftCrop.x * 100}%; top: ${draftCrop.y * 100}%; width: ${
							draftCrop.width * 100
						}%; height: ${draftCrop.height * 100}%;`}
						role="presentation"
						onmousedown={(event) => beginCropDrag('move', event)}
					>
						{#each [1, 2] as index (index)}
							<div
								class="absolute left-0 h-px w-full bg-foreground/40"
								style={`top: ${(index / 3) * 100}%`}
							></div>
							<div
								class="absolute top-0 h-full w-px bg-foreground/40"
								style={`left: ${(index / 3) * 100}%`}
							></div>
						{/each}

						{#each [{ id: 'nw', top: 0, left: 0 }, { id: 'n', top: 0, left: 50 }, { id: 'ne', top: 0, left: 100 }, { id: 'e', top: 50, left: 100 }, { id: 'se', top: 100, left: 100 }, { id: 's', top: 100, left: 50 }, { id: 'sw', top: 100, left: 0 }, { id: 'w', top: 50, left: 0 }] as handle (handle.id)}
							<span
								onmousedown={(event) => beginCropDrag(handle.id as DragHandle, event)}
								class="absolute block size-3 rounded-full border border-foreground bg-foreground"
								style={`cursor: ${getHandleCursor(handle.id, isSideRotation)}; top: calc(${handle.top}% - 6px); left: calc(${handle.left}% - 6px);`}
								role="presentation"
							></span>
						{/each}
					</div>
				{/if}
			</div>
		</div>
		{#if !cropMode && (!isPlaying || isHovering)}
			<div
				class="absolute inset-0 z-10 flex cursor-pointer items-center justify-center"
				onclick={(e) => {
					e.stopPropagation();
					togglePlay();
				}}
				role="presentation"
			>
				<div class="absolute inset-0 bg-black/40" transition:fade={{ duration: 100 }}></div>
				<div
					class="relative flex size-16 items-center justify-center rounded-full bg-gray-alpha-200 text-foreground shadow-sm backdrop-blur-md"
					style="transform-origin: center; will-change: opacity; transform: translateZ(0);"
					transition:fade={{ duration: 100 }}
				>
					{#if isPlaying}
						<IconPause2 size={24} />
					{:else}
						<IconPlay size={24} />
					{/if}
				</div>
			</div>
		{/if}
		{#if cropMode && draftCrop}
			<div
				class="pointer-events-auto absolute bottom-4 left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-md border border-gray-alpha-200 bg-background p-1 text-[10px] font-medium shadow-xl"
			>
				{#each ASPECT_OPTIONS as option (option.id)}
					<Button
						size="sm"
						variant={cropAspect === option.id ? 'selected' : 'ghost'}
						onclick={() => selectAspect(option.id)}
					>
						{option.labelKey ? $_(option.labelKey) : option.display}
					</Button>
				{/each}
				<div class="h-4 w-px bg-gray-alpha-200"></div>
				<Button size="sm" variant="ghost" onclick={resetCropSelection}>{$_('crop.reset')}</Button>
				<Button size="sm" onclick={applyCrop} disabled={!draftCrop || !hasCropDimensions}>
					{$_('crop.apply')}
				</Button>
			</div>
		{/if}
	</div>

	<div class="mt-4 px-2">
		<div
			class="relative mb-6 h-8 select-none"
			class:cursor-pointer={!controlsDisabled}
			class:pointer-events-none={controlsDisabled}
			class:opacity-50={controlsDisabled}
			bind:this={sliderRef}
			role="presentation"
			onmousedown={(e) => !controlsDisabled && e.target === sliderRef && seekTo(e)}
		>
			<div
				class="pointer-events-none absolute top-1/2 left-0 h-1.5 w-full -translate-y-1/2 overflow-hidden rounded-full bg-gray-alpha-100"
			>
				<div
					class="absolute h-full bg-gray-alpha-200"
					style="left: {(startValue / duration) * 100}%; right: {100 -
						(endValue / duration) * 100}%;"
				></div>
			</div>

			<div
				class="pointer-events-none absolute top-1/2 z-10 h-4 w-0.5 -translate-y-1/2 bg-gray-alpha-600"
				style="left: {(currentTime / duration) * 100}%"
			></div>

			<div
				class="absolute top-1/2 z-20 -ml-2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-full border border-blue-600 bg-background shadow-lg"
				class:cursor-ew-resize={!controlsDisabled}
				style="left: {(startValue / duration) * 100}%"
				role="presentation"
				onmousedown={(e) => !controlsDisabled && handleMouseDown(e, 'start')}
			>
				<div class="h-1.5 w-1.5 rounded-full bg-blue-600"></div>
			</div>

			<div
				class="absolute top-1/2 z-20 -ml-2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-full border border-blue-600 bg-background shadow-lg"
				class:cursor-ew-resize={!controlsDisabled}
				style="left: {(endValue / duration) * 100}%"
				role="presentation"
				onmousedown={(e) => !controlsDisabled && handleMouseDown(e, 'end')}
			>
				<div class="h-1.5 w-1.5 rounded-full bg-blue-600"></div>
			</div>
		</div>

		<div class="relative flex flex-wrap items-end justify-between gap-4">
			<div class="flex flex-wrap gap-4">
				<div class="space-y-1.5">
					<Label>{$_('trim.startTime')}</Label>
					<TimecodeInput
						class="w-32"
						value={startValue}
						disabled={controlsDisabled}
						onchange={(val) => {
							if (val >= 0 && val < endValue) {
								startValue = val;
								if (videoRef) videoRef.currentTime = startValue;
								commitTrimValues();
							}
						}}
					/>
				</div>
				<div class="space-y-1.5">
					<Label>{$_('trim.endTime')}</Label>
					<TimecodeInput
						class="w-32"
						value={endValue}
						disabled={controlsDisabled}
						onchange={(val) => {
							if (val > startValue && val <= duration) {
								endValue = val;
								if (videoRef) videoRef.currentTime = endValue;
								commitTrimValues();
							}
						}}
					/>
				</div>
				<div class="space-y-1.5">
					<Label>{$_('trim.duration')}</Label>
					<div class="py-1.5 text-[11px] tracking-wide text-foreground">
						{formatTime(endValue - startValue)}
					</div>
				</div>
			</div>
			<div class="absolute right-0 bottom-0 flex gap-2">
				<Button
					size="icon"
					variant="ghost"
					title={$_('video.rotation')}
					onclick={handleRotateToggle}
					disabled={controlsDisabled}
				>
					<IconRotateCw size={16} />
				</Button>
				<Button
					size="icon"
					variant={flipHorizontal ? 'selected' : 'ghost'}
					title={$_('video.flipHorizontal')}
					onclick={() => toggleFlip('horizontal')}
					disabled={controlsDisabled}
				>
					<FlipHorizontalIcon size={16} />
				</Button>
				<Button
					size="icon"
					variant={flipVertical ? 'selected' : 'ghost'}
					title={$_('video.flipVertical')}
					onclick={() => toggleFlip('vertical')}
					disabled={controlsDisabled}
				>
					<FlipVerticalIcon size={16} />
				</Button>
				<Button
					size="icon"
					variant={cropMode ? 'selected' : appliedCrop ? 'selected' : 'ghost'}
					title={$_('crop.enter')}
					onclick={toggleCropMode}
					disabled={controlsDisabled || !hasCropDimensions}
				>
					<CropIcon size={16} />
				</Button>
			</div>
		</div>
	</div>
</div>
