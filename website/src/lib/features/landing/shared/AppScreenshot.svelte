<script lang="ts">
	import { cn } from '$lib/utils';

	type Crop = { x: number; y: number; width: number; height: number; radius: number };

	let {
		src,
		lightSrc,
		alt,
		width,
		height,
		crop,
		framed = true,
		fade = false,
		priority = false,
		class: className
	}: {
		src: string;
		lightSrc?: string;
		alt: string;
		width: number;
		height: number;
		crop: Crop;
		framed?: boolean;
		fade?: boolean;
		priority?: boolean;
		class?: string;
	} = $props();

	// Match the docs: app.html initializes .dark; themeStore keeps it in sync.
	// Keeping both sources in the markup avoids mismatched SSR image attributes.
	const sources = $derived(
		lightSrc
			? [
					{ src: lightSrc, theme: 'light' },
					{ src, theme: 'dark' }
				]
			: [{ src, theme: 'default' }]
	);
</script>

<!-- Crop coordinates remove the baked native edges. A separate shadow layer
     supplies the edge and highlight without being covered by the bitmap.
     The compensated gutter lets the fade include that shadow without changing
     the image width or the caller's height limit. -->
<div
	data-capture-viewport
	data-fade={fade ? '' : undefined}
	class={cn('w-full', framed ? 'flow-root' : 'overflow-hidden', !framed && className)}
>
	<div
		class={cn(
			framed && 'box-content -m-6 w-full p-6 md:-m-8 md:p-8',
			fade && 'overflow-hidden mask-b-from-65% mask-b-to-90% mask-alpha',
			framed && className
		)}
	>
		<div
			data-app-screenshot
			class="relative w-full"
			style:aspect-ratio={`${crop.width} / ${crop.height}`}
			style:border-radius={`${(100 * crop.radius) / crop.width}% / ${(100 * crop.radius) / crop.height}%`}
		>
			<div class="absolute inset-0 overflow-hidden" style:border-radius="inherit">
				{#each sources as source (source.theme)}
					<img
						src={source.src}
						{alt}
						{width}
						{height}
						loading={priority ? 'eager' : 'lazy'}
						fetchpriority={priority ? 'high' : 'auto'}
						data-capture-theme={source.theme === 'default' ? undefined : source.theme}
						class={[
							'absolute top-0 left-0 h-auto max-w-none',
							source.theme === 'light'
								? 'block dark:hidden'
								: source.theme === 'dark'
									? 'hidden dark:block'
									: 'block'
						]}
						style:width={`${(100 * width) / crop.width}%`}
						style:transform={`translate(${(-100 * crop.x) / width}%, ${(-100 * crop.y) / height}%)`}
					/>
				{/each}
			</div>
			{#if framed}
				<div
					data-capture-shadow
					aria-hidden="true"
					class="pointer-events-none absolute inset-0 shadow-lg"
					style:border-radius="inherit"
				></div>
			{/if}
		</div>
	</div>
</div>
