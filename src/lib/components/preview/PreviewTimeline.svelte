<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import TimecodeInput from '$lib/components/ui/TimecodeInput.svelte';
	import { IconPause2, IconPlay } from '$lib/icons';
	import { _ } from '$lib/i18n';
	import { cn } from '$lib/utils/cn';
	import type { PreviewPlaybackController } from '$lib/features/preview';
	import { formatTime } from '$lib/features/preview';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	let {
		playback,
		trimDisabled,
		isImage
	}: {
		playback: PreviewPlaybackController;
		trimDisabled: boolean;
		isImage: boolean;
	} = $props();

	let sliderRef = $state<HTMLDivElement | undefined>();
	let isHovered = $state(false);
	let hoverX = $state(0);

	$effect(() => {
		playback.setSliderElement(sliderRef);
	});

	function updateHoverPosition(event: MouseEvent) {
		if (!sliderRef || trimDisabled || playback.duration <= 0) return;

		const rect = sliderRef.getBoundingClientRect();
		if (rect.width <= 0) return;

		hoverX = Math.max(0, Math.min(event.clientX - rect.left, rect.width));
	}
</script>

<div class="mt-4 px-2">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-end">
		<div class="flex flex-wrap gap-4 lg:shrink-0 lg:justify-end">
			<div class="space-y-1.5">
				<Label>{$_('trim.startTime')}</Label>
				{#if isImage}
					<div class="flex h-7.5 w-32 items-center text-[10px] text-frame-gray-600">
						--:--:--.---
					</div>
				{:else}
					<TimecodeInput
						class="w-32"
						value={playback.startValue}
						disabled={trimDisabled}
						onchange={(value) => {
							if (value >= 0 && value < playback.endValue) {
								playback.setStartValue(value);
								if (playback.mediaElement) playback.mediaElement.currentTime = value;
								playback.commitTrimValues();
							}
						}}
					/>
				{/if}
			</div>

			<div class="space-y-1.5">
				<Label>{$_('trim.endTime')}</Label>
				{#if isImage}
					<div class="flex h-7.5 w-32 items-center text-[10px] text-frame-gray-600">
						--:--:--.---
					</div>
				{:else}
					<TimecodeInput
						class="w-32"
						value={playback.endValue}
						disabled={trimDisabled}
						onchange={(value) => {
							if (value > playback.startValue && value <= playback.duration) {
								playback.setEndValue(value);
								if (playback.mediaElement) playback.mediaElement.currentTime = value;
								playback.commitTrimValues();
							}
						}}
					/>
				{/if}
			</div>

			<div class="space-y-1.5">
				<Label>{$_('trim.duration')}</Label>
				<div class="flex h-7.5 items-center text-[10px] text-foreground">
					{isImage ? '--:--:--.---' : formatTime(playback.endValue - playback.startValue)}
				</div>
			</div>
		</div>
		<div class="min-w-0 flex-1 space-y-1.5">
			<Label>{$_('trim.label')}</Label>
			<div
				class={cn(
					'relative h-7.5 select-none',
					trimDisabled ? 'pointer-events-none opacity-50' : 'cursor-pointer'
				)}
				bind:this={sliderRef}
				role="presentation"
				onmouseenter={() => (isHovered = true)}
				onmouseleave={() => (isHovered = false)}
				onmousemove={updateHoverPosition}
				onmousedown={(event) => {
					if (!trimDisabled) {
						playback.seekTo(event);
					}
				}}
			>
				<div
					class="pointer-events-none absolute top-1/2 left-0 h-1.5 w-full -translate-y-1/2 rounded-[1.5px] bg-frame-gray-100"
				></div>

				<div
					class="pointer-events-none absolute top-1/2 h-1.5 -translate-y-1/2 rounded-[1px] bg-foreground"
					style={`right: ${100 - playback.toTimelinePercent(playback.endValue)}%; left: ${playback.toTimelinePercent(playback.startValue)}%;`}
				></div>

				<div
					class="pointer-events-none absolute top-1/2 z-30 h-4 w-px -translate-x-1/2 -translate-y-1/2 bg-foreground"
					style={`left: ${playback.toTimelinePercent(playback.currentTime)}%`}
				></div>

				<div
					class={cn(
						'absolute top-1/2 z-20 h-7.5 w-5 -translate-x-1/2 -translate-y-1/2',
						!trimDisabled && 'cursor-ew-resize'
					)}
					style={`left: ${playback.toTimelinePercent(playback.startValue)}%`}
					role="presentation"
					onmousedown={(event) => {
						event.stopPropagation();
						if (!trimDisabled) playback.beginHandleDrag(event, 'start');
					}}
				></div>

				<div
					class={cn(
						'absolute top-1/2 z-20 h-7.5 w-5 -translate-x-1/2 -translate-y-1/2',
						!trimDisabled && 'cursor-ew-resize'
					)}
					style={`left: ${playback.toTimelinePercent(playback.endValue)}%`}
					role="presentation"
					onmousedown={(event) => {
						event.stopPropagation();
						if (!trimDisabled) playback.beginHandleDrag(event, 'end');
					}}
				></div>

				{#if !trimDisabled && isHovered}
					<div
						class="pointer-events-none absolute top-1/2 z-10 h-4 w-px -translate-x-1/2 -translate-y-1/2 bg-frame-gray-600"
						style={`left: ${hoverX}px;`}
					></div>
				{/if}
			</div>
		</div>
		<div class="space-y-1.5">
			<Label>&nbsp;</Label>
			<Tooltip content={playback.isPlaying ? $_('common.pause') : $_('common.resume')}>
				<Button
					size="icon"
					variant="ghost"
					onclick={() => playback.togglePlay()}
					disabled={trimDisabled}
				>
					{#if playback.isPlaying}
						<IconPause2 size={16} />
					{:else}
						<IconPlay size={16} />
					{/if}
				</Button>
			</Tooltip>
		</div>
	</div>
</div>
