<script lang="ts">
	import { ArrowRightIcon } from '$lib/icons';
	import AudioIllustration from './illustrations/AudioIllustration.svelte';
	import ImageIllustration from './illustrations/ImageIllustration.svelte';
	import VideoIllustration from './illustrations/VideoIllustration.svelte';

	const features = [
		{
			id: 'video',
			title: 'Video',
			description: 'Choose codecs, set resolution and frame rate, and tune quality for your output.'
		},
		{
			id: 'audio',
			title: 'Audio',
			description: 'Extract tracks, adjust bitrate and channels, and apply audio filters.'
		},
		{
			id: 'images',
			title: 'Images',
			description: 'Resize, crop, and convert images with format-specific compression controls.'
		}
	];
</script>

<div class="grid lg:grid-cols-3" data-media-feature-cards>
	{#each features as feature, index (feature.id)}
		<article
			class={[
				'flex min-w-0 flex-col px-6 pb-8 lg:px-7',
				index > 0 && 'border-t border-border lg:border-t-0 lg:border-l'
			]}
			aria-labelledby={`media-feature-${feature.id}`}
			data-media-feature={feature.id}
		>
			<div
				class="relative grid h-64 place-items-center overflow-hidden"
				aria-hidden="true"
				data-media-illustration={feature.id}
			>
				{#if feature.id === 'video'}
					<VideoIllustration />
				{:else if feature.id === 'audio'}
					<AudioIllustration />
				{:else}
					<ImageIllustration />
				{/if}
			</div>
			<div class="flex flex-1 flex-col">
				<h3
					id={`media-feature-${feature.id}`}
					class="text-lg leading-snug font-medium tracking-tight text-foreground"
				>
					{feature.title}
				</h3>
				<p class="mt-3 max-w-sm text-sm leading-relaxed text-muted-foreground">
					{feature.description}
				</p>
				<a
					href={`/docs/${feature.id}`}
					class="mt-auto inline-flex w-fit items-center gap-2 pt-6 text-xs text-muted-foreground transition-colors duration-150 ease-out hover:text-foreground"
					>Explore {feature.id}<ArrowRightIcon class="size-3.5" aria-hidden="true" /></a
				>
			</div>
		</article>
	{/each}
</div>
