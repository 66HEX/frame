<script lang="ts">
	import AppScreenshot from '$lib/features/landing/shared/AppScreenshot.svelte';

	// A three-pixel inset removes the baked panel rings; equal crops retain matched scaling.
</script>

<figure
	data-workflow-stage
	aria-label="A batch of video files and Frame’s preset library"
	class="grid gap-8 md:grid-cols-2"
>
	<div data-batch-queue class="mx-auto w-full min-w-0 max-w-xl md:max-w-none">
		<AppScreenshot
			src="/media/landing/batch-queue-dark.webp"
			lightSrc="/media/landing/batch-queue-light.webp"
			alt="Three video files selected in Frame’s source queue"
			width={1568}
			height={480}
			crop={{ x: 19, y: 11, width: 751, height: 458, radius: 28 }}
			fade
		/>
	</div>
	<div data-preset-library class="mx-auto w-full min-w-0 max-w-xl md:max-w-none">
		<AppScreenshot
			src="/media/landing/presets-dark.webp"
			lightSrc="/media/landing/presets-light.webp"
			alt="Frame’s preset library with Balanced MP4 applied"
			width={784}
			height={1320}
			crop={{ x: 14, y: 3, width: 751, height: 458, radius: 28 }}
			fade
		/>
	</div>
</figure>
