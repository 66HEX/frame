<script lang="ts">
	import AppScreenshot from '$lib/features/landing/shared/AppScreenshot.svelte';
</script>

<figure
	data-hero-preview
	aria-label="The Frame workspace"
	class="-my-8 -ms-8 -me-6 h-112 overflow-hidden p-8 md:-mx-10 md:h-auto"
>
	<!-- The padding contains the shadow while keeping the screenshot aligned.
	     Paint the shadow above the image so its inset highlights stay visible. -->
	<div
		data-hero-capture
		class="relative w-xl after:pointer-events-none after:absolute after:inset-0 after:rounded-xl after:shadow-lg md:w-auto"
	>
		<AppScreenshot
			framed={false}
			class="rounded-xl"
			src="/media/landing/hero-dark.webp"
			lightSrc="/media/landing/hero-light.webp"
			alt="Frame’s native workspace with a video preview, a queue of source files, and detailed output settings"
			width={2400}
			height={1600}
			crop={{ x: 0, y: 0, width: 2400, height: 1600, radius: 24 }}
			priority
		/>
		<img
			src="/media/macos-traffic-lights.svg"
			alt=""
			aria-hidden="true"
			width="2400"
			height="1600"
			class="pointer-events-none absolute inset-0 size-full"
		/>
	</div>
</figure>
