<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { Play, Pause } from 'lucide-svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import TimecodeInput from '$lib/components/ui/TimecodeInput.svelte';
	import { _ } from '$lib/i18n';

	let {
		filePath,
		initialStartTime,
		initialEndTime,
		onSave,
		onCancel
	}: {
		filePath: string;
		initialStartTime?: string;
		initialEndTime?: string;
		onSave: (start?: string, end?: string) => void;
		onCancel: () => void;
	} = $props();

	let videoSrc = $state('');

	onMount(() => {
		videoSrc = convertFileSrc(filePath);

		if (initialStartTime) startValue = parseTimeToSeconds(initialStartTime);
	});

	let videoRef: HTMLVideoElement | undefined = $state();
	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);

	let startValue = $state(0);
	let endValue = $state(0);

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

	onMount(() => {
		if (initialStartTime) startValue = parseTimeToSeconds(initialStartTime);
	});

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

	function handleSave() {
		const startStr = startValue > 0 ? formatTime(startValue) : undefined;
		const endStr = endValue < duration ? formatTime(endValue) : undefined;
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
		} else {
			endValue = Math.max(time, startValue + 1);
			if (videoRef) videoRef.currentTime = endValue;
		}
	}

	function handleMouseUp() {
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

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="absolute inset-0 z-50 flex items-center justify-center bg-background/60 backdrop-blur-sm"
	transition:fade={{ duration: 100 }}
	onclick={onCancel}
>
	<div
		class="flex h-[85vh] w-[80vw] flex-col overflow-hidden rounded-xl border border-ds-blue-600 bg-ds-blue-900/20 p-3 shadow-2xl backdrop-blur-sm"
		transition:scale={{ start: 1.025, duration: 100, opacity: 1 }}
		onclick={(e) => e.stopPropagation()}
	>
		<div class="relative flex-1 rounded-lg bg-black">
			<video
				bind:this={videoRef}
				src={videoSrc}
				class="h-full w-full rounded-lg bg-black object-cover"
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
					class="pointer-events-none absolute inset-0 flex items-center justify-center rounded-lg bg-background/60"
				>
					<div
						class="flex size-16 items-center justify-center rounded-full bg-gray-alpha-100 backdrop-blur-md"
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
				>
					<div class="h-1.5 w-1.5 rounded-full bg-ds-blue-600"></div>
				</div>

				<div
					class="absolute top-1/2 z-20 -ml-2 flex h-4 w-4 -translate-y-1/2 cursor-ew-resize items-center justify-center rounded-full border border-ds-blue-600 bg-background shadow-lg"
					style="left: {(endValue / duration) * 100}%"
					onmousedown={(e) => handleMouseDown(e, 'end')}
				>
					<div class="h-1.5 w-1.5 rounded-full bg-ds-blue-600"></div>
				</div>
			</div>

			<div class="flex items-end justify-between gap-4">
				<div class="flex gap-4">
					<div class="space-y-1.5">
						<Label>{$_('trim.startTime')}</Label>
						<TimecodeInput
							value={startValue}
							onchange={(val) => {
								if (val >= 0 && val < endValue) {
									startValue = val;
									if (videoRef) videoRef.currentTime = startValue;
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

				<div class="flex items-center gap-2">
					<Button variant="ghost" size="icon" onclick={togglePlay}>
						{#if isPlaying}
							<Pause size={14} />
						{:else}
							<Play size={14} />
						{/if}
					</Button>
					<div class="bg-gray-alpha-200 mx-2 h-4 w-px"></div>
					<Button variant="outline" onclick={onCancel}>{$_('trim.cancel')}</Button>
					<Button onclick={handleSave}>{$_('trim.save')}</Button>
				</div>
			</div>
		</div>
	</div>
</div>
