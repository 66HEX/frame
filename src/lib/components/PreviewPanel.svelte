<script lang="ts">
	import { onMount } from 'svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { Play } from 'lucide-svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import TimecodeInput from '$lib/components/ui/TimecodeInput.svelte';
	import { _ } from '$lib/i18n';

	let {
		filePath,

		initialStartTime,
		initialEndTime,
		rotation = '0',
		flipHorizontal = false,
		flipVertical = false,
		onSave
	}: {
		filePath: string;
		initialStartTime?: string;
		initialEndTime?: string;
		rotation?: '0' | '90' | '180' | '270';
		flipHorizontal?: boolean;
		flipVertical?: boolean;
		onSave: (start?: string, end?: string) => void;
	} = $props();

	let videoSrc = $state('');
	let containerRef: HTMLDivElement | undefined = $state();
	let containerWidth = $state(0);
	let containerHeight = $state(0);

	let isSideRotation = $derived(rotation === '90' || rotation === '270');

	let videoStyle = $derived(
		isSideRotation && containerWidth && containerHeight
			? `width: ${containerHeight}px; height: ${containerWidth}px;`
			: 'width: 100%; height: 100%;'
	);

	let transformStyle = $derived(
		[`rotate(${rotation}deg)`, flipHorizontal ? 'scaleX(-1)' : '', flipVertical ? 'scaleY(-1)' : '']
			.filter(Boolean)
			.join(' ')
	);

	onMount(() => {
		videoSrc = convertFileSrc(filePath);
		if (initialStartTime) startValue = parseTimeToSeconds(initialStartTime);
		if (containerRef) {
			containerWidth = containerRef.clientWidth;

			containerHeight = containerRef.clientHeight;
		}
		const ro = new ResizeObserver((entries) => {
			for (const entry of entries) {
				containerWidth = entry.contentRect.width;

				containerHeight = entry.contentRect.height;
			}
		});
		if (containerRef) ro.observe(containerRef);
		return () => ro.disconnect();
	});

	let videoRef: HTMLVideoElement | undefined = $state();
	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let startValue = $state(0);
	let endValue = $state(0);
	let previousInitialStart: string | undefined;
	let previousInitialEnd: string | undefined;

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
			} else {
				endValue = duration || 0;
			}
		}
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

	let sliderRef: HTMLDivElement | undefined = $state();
	let dragging: 'start' | 'end' | null = null;

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

		if (dragging === 'start') {
			startValue = Math.min(time, endValue - 1);
			if (videoRef) videoRef.currentTime = startValue;
			commitTrimValues();
		} else {
			endValue = Math.max(time, startValue + 1);
			if (videoRef) videoRef.currentTime = endValue;
			commitTrimValues();
		}
	}

	function handleMouseUp() {
		if (dragging) {
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
	}
</script>

<div
	class="flex h-full flex-col overflow-hidden rounded-xl border border-gray-alpha-100 bg-gray-alpha-100 p-4"
>
	<div
		class="border-gray-alpha-200 relative flex min-h-0 flex-1 items-center justify-center rounded-lg border bg-background"
		bind:this={containerRef}
	>
		<video
			bind:this={videoRef}
			src={videoSrc}
			class="block overflow-hidden rounded-lg bg-background object-contain"
			style={videoStyle}
			style:transform={transformStyle}
			onloadedmetadata={handleMetadata}
			ontimeupdate={handleTimeUpdate}
			onplay={() => (isPlaying = true)}
			onpause={() => (isPlaying = false)}
			onclick={togglePlay}
		>
			<track kind="captions" />
		</video>

		{#if !isPlaying}
			<div
				class="pointer-events-none absolute inset-0 flex items-center justify-center rounded-lg bg-background/80"
			>
				<div
					class="bg-gray-alpha-200 flex size-16 items-center justify-center rounded-full backdrop-blur-md"
				>
					<Play size={24} fill="currentColor" class="ml-1" />
				</div>
			</div>
		{/if}
	</div>

	<div class="mt-4 px-2">
		<div
			class="relative mb-6 h-8 cursor-pointer select-none"
			bind:this={sliderRef}
			onmousedown={(e) => e.target === sliderRef && seekTo(e)}
			role="presentation"
		>
			<div
				class="pointer-events-none absolute top-1/2 left-0 h-1.5 w-full -translate-y-1/2 overflow-hidden rounded-full bg-gray-alpha-100"
			>
				<div
					class="bg-gray-alpha-200 absolute h-full"
					style="left: {(startValue / duration) * 100}%; right: {100 -
						(endValue / duration) * 100}%;"
				></div>
			</div>

			<div
				class="bg-gray-alpha-600 pointer-events-none absolute top-1/2 z-10 h-4 w-0.5 -translate-y-1/2"
				style="left: {(currentTime / duration) * 100}%"
			></div>

			<div
				class="absolute top-1/2 z-20 -ml-2 flex h-4 w-4 -translate-y-1/2 cursor-ew-resize items-center justify-center rounded-full border border-ds-blue-600 bg-background shadow-lg"
				style="left: {(startValue / duration) * 100}%"
				onmousedown={(e) => handleMouseDown(e, 'start')}
				role="presentation"
			>
				<div class="h-1.5 w-1.5 rounded-full bg-ds-blue-600"></div>
			</div>

			<div
				class="absolute top-1/2 z-20 -ml-2 flex h-4 w-4 -translate-y-1/2 cursor-ew-resize items-center justify-center rounded-full border border-ds-blue-600 bg-background shadow-lg"
				style="left: {(endValue / duration) * 100}%"
				onmousedown={(e) => handleMouseDown(e, 'end')}
				role="presentation"
			>
				<div class="h-1.5 w-1.5 rounded-full bg-ds-blue-600"></div>
			</div>
		</div>

		<div class="flex flex-wrap items-end justify-between gap-4">
			<div class="flex flex-wrap gap-4">
				<div class="space-y-1.5">
					<Label>{$_('trim.startTime')}</Label>
					<TimecodeInput
						value={startValue}
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
						value={endValue}
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
					<div class="text-gray-alpha-600 py-1.5 font-mono text-[10px] tracking-wide">
						{formatTime(endValue - startValue)}
					</div>
				</div>
			</div>
		</div>
	</div>
</div>
