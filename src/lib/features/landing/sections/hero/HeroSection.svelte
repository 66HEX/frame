<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ArrowRightIcon } from '$lib/icons';
	import TextLink from '../../shared/TextLink.svelte';
	import FramePreview from './FramePreview.svelte';
</script>

<section
	aria-labelledby="hero-title"
	class="mx-auto w-full max-w-7xl px-6 pt-14 md:px-10 md:pt-20 lg:pt-26"
>
	<h1
		id="hero-title"
		class="max-w-4xl text-4xl leading-none font-medium tracking-tight md:text-5xl lg:text-6xl"
	>
		Every format.<br />One workspace.
	</h1>
	<div
		class="mt-6 flex flex-col items-start justify-between gap-6 md:mt-8 md:gap-8 lg:flex-row lg:items-end"
	>
		<p class="text-base leading-relaxed text-pretty text-muted-foreground">
			Video, audio, and images. All in a native app.<br class="hidden sm:block" /> The power of FFmpeg,
			with a little more control.
		</p>
		<div class="flex flex-wrap items-center gap-4">
			<Button href="#download" size="lg">Download Frame <ArrowRightIcon /></Button>
			<TextLink href="/docs">Read the docs<ArrowRightIcon class="size-4" /></TextLink>
		</div>
	</div>
	<div class="mt-11 md:mt-14 lg:mt-18"><FramePreview /></div>
	<div
		class="flex flex-col items-start justify-between gap-3 pt-6 md:flex-row md:items-center md:gap-6 md:pt-8 md:pb-12"
	>
		<p class="text-sm leading-normal text-muted-foreground">
			macOS<span class="px-3.5 text-muted-foreground md:px-5">·</span>Windows<span
				class="px-3.5 text-muted-foreground md:px-5">·</span
			>Linux
		</p>
	</div>
</section>
