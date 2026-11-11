<script lang="ts">
	import AppScreenshot from '../../shared/AppScreenshot.svelte';

	// Isolate General inside the native panel edge and stop before the Video divider.
	const crop = { x: 1614, y: 314, width: 751, height: 626, radius: 28 };
</script>

<figure
	class="mx-auto w-full min-w-0 max-w-md"
	aria-label="Frame’s native controls for processing and saving files locally"
	data-local-processing
>
	<AppScreenshot
		src="/media/landing/workflow-dark.webp"
		lightSrc="/media/landing/workflow-light.webp"
		alt="Frame’s General panel with processing mode, output filename, format, and a note that output is saved to a local folder"
		width={2400}
		height={1600}
		{crop}
	/>
</figure>
