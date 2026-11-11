<script lang="ts">
	import { ArrowRightIcon } from '$lib/icons';
	import AppScreenshot from '$lib/features/landing/shared/AppScreenshot.svelte';
	import TextLink from '$lib/features/landing/shared/TextLink.svelte';

	// Measured source bounds sit inside the baked panel rings. The subtitle crop
	// stops before the next section's divider; the other captures continue below.
	const crops = {
		audio: { x: 14, y: 3, width: 751, height: 1037, radius: 28 },
		metadata: { x: 14, y: 8, width: 751, height: 1072, radius: 28 },
		subtitles: { x: 14, y: 10, width: 751, height: 688, radius: 28 }
	};

	const views = [
		{
			id: 'audio',
			src: 'audio-inspector',
			height: 1040,
			title: 'Give sound its space.',
			text: 'Choose the codec, channels, and bitrate. Export a recording, extract a soundtrack, or keep a lossless copy.',
			href: '/docs/audio',
			link: 'Explore audio controls',
			alt: 'Frame’s audio output panel with codec, bitrate, quality, and channel controls'
		},
		{
			id: 'metadata',
			src: 'metadata-inspector',
			height: 1080,
			title: 'Keep the right details.',
			text: 'Review the metadata that travels with your files. Keep the tags you need, edit them, or remove them from your output.',
			href: '/docs/metadata',
			link: 'Explore metadata controls',
			alt: 'Frame’s metadata editor showing editable media tags'
		},
		{
			id: 'subtitles',
			src: 'subtitles-inspector',
			height: 1040,
			title: 'Every word, accounted for.',
			text: 'Choose the subtitle tracks for your output or bring your own subtitle file. Keep the right language with the right video.',
			href: '/docs/subtitles',
			link: 'Explore subtitle controls',
			alt: 'Frame’s subtitle controls with track selection and an option to add an external subtitle file'
		}
	] as const;
</script>

<div class="grid gap-16 md:gap-20 lg:grid-cols-3 lg:gap-8" data-output-details>
	{#each views as view (view.id)}
		<article
			data-detail-layout={view.id}
			aria-labelledby={`output-detail-${view.id}`}
			class="grid min-w-0 gap-8 md:grid-cols-2 md:items-center md:gap-16 lg:flex lg:flex-col lg:items-stretch lg:gap-8"
		>
			<div data-detail-image class="h-72">
				<div data-detail-capture class="mx-auto w-full max-w-xs lg:max-w-none">
					<AppScreenshot
						src={`/media/landing/${view.src}-dark.webp`}
						lightSrc={`/media/landing/${view.src}-light.webp`}
						alt={view.alt}
						width={784}
						height={view.height}
						crop={crops[view.id]}
						class="max-h-72"
						fade
					/>
				</div>
			</div>
			<div data-detail-copy class="flex min-w-0 flex-col lg:flex-1">
				<h3
					id={`output-detail-${view.id}`}
					class="text-xl leading-snug font-medium tracking-tight text-balance"
				>
					{view.title}
				</h3>
				<p class="mt-4 max-w-md text-base leading-relaxed text-pretty text-muted-foreground">
					{view.text}
				</p>
				<div class="mt-5 lg:mt-auto lg:pt-5">
					<TextLink href={view.href}
						>{view.link}<ArrowRightIcon class="size-4" aria-hidden="true" /></TextLink
					>
				</div>
			</div>
		</article>
	{/each}
</div>
