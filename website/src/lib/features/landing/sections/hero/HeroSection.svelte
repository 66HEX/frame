<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ArrowRightIcon } from '$lib/icons';
	import TextLink from '../../shared/TextLink.svelte';
	import FramePreview from './FramePreview.svelte';
	import HeroShader from './HeroShader.svelte';
</script>

<section
	aria-labelledby="hero-title"
	class="relative isolate mx-3 mb-12 overflow-hidden rounded-lg pb-16 md:mx-4 md:mb-20 md:pb-24"
>
	<HeroShader />
	<div class="mx-auto w-full max-w-7xl px-6 pt-14 md:px-10 md:pt-20 lg:pt-26">
		<h1
			id="hero-title"
			class="mx-auto max-w-4xl text-center text-title-5 font-medium text-balance sm:text-title-7 lg:text-title-8"
		>
			Every format.<br />One workspace.
		</h1>
		<div class="mt-6 flex flex-col items-center gap-6 text-center md:mt-8 md:gap-8">
			<p class="max-w-2xl text-regular text-pretty text-muted-foreground sm:text-large">
				Video, audio, and images. All in a native app.<br class="hidden sm:block" /> The power of FFmpeg,
				with a little more control.
			</p>
			<div class="flex flex-wrap items-center justify-center gap-4">
				<Button href="#download" size="default">Download Frame <ArrowRightIcon /></Button>
				<TextLink href="/docs">Read the docs<ArrowRightIcon class="size-4" /></TextLink>
			</div>
		</div>
		<div class="mt-11 md:mt-14 lg:mt-18"><FramePreview /></div>
	</div>
</section>
