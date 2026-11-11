<script lang="ts">
	import AppScreenshot from '$lib/features/landing/shared/AppScreenshot.svelte';
</script>

<figure
	data-preview-stage
	aria-label="Frame’s crop editor, video output controls, and trim timeline"
	class="relative isolate h-auto min-h-0 pb-6 md:aspect-2/1 md:min-h-80 md:pb-0"
>
	<div data-preview-canvas class="relative top-0 left-0 w-full md:absolute md:w-4/5">
		<AppScreenshot
			src="/media/landing/preview-canvas-dark.webp"
			lightSrc="/media/landing/preview-canvas-light.webp"
			alt="Adjustable crop handles around an alpine lake in Frame’s preview"
			width={1568}
			height={992}
			crop={{ x: 20, y: 20, width: 1539, height: 952, radius: 24 }}
		/>
	</div>
	<div data-preview-inspector class="absolute top-1/16 right-0 hidden w-3/10 md:block">
		<AppScreenshot
			src="/media/landing/video-inspector-dark.webp"
			lightSrc="/media/landing/video-inspector-light.webp"
			alt="Native controls for resolution, video encoder, pixel format, and quality"
			width={784}
			height={1180}
			crop={{ x: 15, y: 4, width: 749, height: 1168, radius: 24 }}
		/>
	</div>
	<div
		data-preview-trim
		class="relative bottom-0 left-1/8 -mt-3 w-3/4 md:absolute md:left-1/6 md:mt-0 md:w-3/8"
	>
		<AppScreenshot
			src="/media/landing/trim-detail-dark.webp"
			lightSrc="/media/landing/trim-detail-light.webp"
			alt="Trim a clip with start and end points on Frame’s timeline"
			width={832}
			height={320}
			crop={{ x: 105, y: 52, width: 632, height: 144, radius: 24 }}
		/>
	</div>
</figure>
