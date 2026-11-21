<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Dialog as DialogPrimitive } from 'bits-ui';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as Card from '$lib/components/ui/card';
	import { XmarkIcon } from '$lib/icons';
	import { cn } from '$lib/utils';
	import { MediaLightbox } from './media-lightbox.svelte';
	import { decodeImage } from './media-readiness';

	type Props = { scene: string; alt: string; caption: string };
	let { scene, alt, caption }: Props = $props();

	let mediaContainer = $state<HTMLDivElement | null>(null);
	let trigger = $state<HTMLButtonElement | null>(null);
	let closeButton = $state<HTMLButtonElement | null>(null);
	let viewport = $state<HTMLDivElement | undefined>();
	let overlay = $state<HTMLElement | null>(null);
	let content = $state<HTMLElement | null>(null);

	const sources = $derived({
		light: `/media/docs/${scene}-light.webp`,
		dark: `/media/docs/${scene}-dark.webp`
	});

	function visibleImage(container: HTMLElement | null) {
		return [...(container?.querySelectorAll('img') ?? [])].find(
			(image) => getComputedStyle(image).display !== 'none'
		);
	}

	const lightbox = new MediaLightbox({
		elements: () => ({
			thumbnail: visibleImage(mediaContainer),
			viewport,
			overlay,
			content,
			trigger
		}),
		frame: async (signal) => {
			const source = visibleImage(mediaContainer);
			if (!source) return null;
			const image = new Image();
			image.src = source.currentSrc || source.src;
			image.alt = '';
			image.setAttribute('aria-hidden', 'true');
			image.dataset.screenshotFlip = scene;
			if (!(await decodeImage(image, signal))) return null;
			return image;
		}
	});

	function changeOpen(value: boolean) {
		lightbox.changeOpen(value);
	}

	onMount(() => lightbox.connect());
	onDestroy(() => lightbox.destroy());
</script>

<figure class="my-8 min-w-0" data-doc-screenshot={scene}>
	<Dialog.Root bind:open={() => lightbox.open, changeOpen}>
		<Card.Root size="sm" class="gap-0 bg-muted p-1">
			<div bind:this={mediaContainer} class="relative">
				<img
					src={sources.light}
					{alt}
					width="2400"
					height="1600"
					loading="lazy"
					decoding="async"
					class={cn(
						'block h-auto w-full rounded-lg shadow-sm transition-none dark:hidden',
						(lightbox.open || lightbox.moving) && 'invisible'
					)}
				/>
				<img
					src={sources.dark}
					{alt}
					width="2400"
					height="1600"
					loading="lazy"
					decoding="async"
					class={cn(
						'hidden h-auto w-full rounded-lg shadow-sm transition-none dark:block',
						(lightbox.open || lightbox.moving) && 'invisible'
					)}
				/>
				<Dialog.Trigger>
					{#snippet child({ props })}
						<button
							{...props}
							bind:this={trigger}
							type="button"
							aria-label={`View full size: ${caption}`}
							class="absolute inset-0 cursor-zoom-in rounded-lg outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
						></button>
					{/snippet}
				</Dialog.Trigger>
			</div>
		</Card.Root>

		<Dialog.Portal>
			<DialogPrimitive.Overlay
				bind:ref={overlay}
				style={`opacity: ${lightbox.opacity}`}
				onclick={() => changeOpen(false)}
				class="fixed inset-0 z-50 bg-black/80"
			/>
			<DialogPrimitive.Content
				bind:ref={content}
				onOpenAutoFocus={(event) => {
					event.preventDefault();
					closeButton?.focus({ preventScroll: true });
				}}
				onCloseAutoFocus={(event) => {
					event.preventDefault();
				}}
				data-screenshot-dialog={scene}
				data-moving={lightbox.moving}
				class="pointer-events-none! fixed inset-0 z-[100] grid place-items-center bg-transparent px-3 py-(--media-inset) outline-none"
				style={`--media-inset: max(4.5rem, calc(env(safe-area-inset-top) + 3.5rem), calc(env(safe-area-inset-bottom) + 0.75rem)); opacity: ${lightbox.opacity}`}
			>
				<Dialog.Title class="sr-only">{caption}</Dialog.Title>
				<Dialog.Description class="sr-only"
					>{alt} Press Escape or click outside the image to close.</Dialog.Description
				>
				<Dialog.Close
					bind:ref={closeButton}
					aria-label="Close screenshot"
					class="pointer-events-auto fixed top-[max(1rem,env(safe-area-inset-top))] right-[max(1rem,env(safe-area-inset-right))] z-10 flex size-11 items-center justify-center rounded-md text-white opacity-80 transition-[opacity,box-shadow,background-color] duration-150 ease-in-out outline-none hover:bg-white/10 hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-3 focus-visible:ring-white/50 motion-reduce:transition-none forced-colors:focus-visible:outline-2 forced-colors:focus-visible:outline-offset-2 forced-colors:focus-visible:outline-[Highlight] forced-colors:focus-visible:outline-solid"
				>
					<XmarkIcon class="size-5" aria-hidden="true" />
				</Dialog.Close>
				<div
					bind:this={viewport}
					class="pointer-events-auto aspect-[3/2] w-[min(calc(100vw-1.5rem),calc((100dvh-2*var(--media-inset))*1.5))] overflow-hidden rounded-xl shadow-lg"
					style:opacity={lightbox.moving ? 0 : 1}
				>
					<img
						src={sources.light}
						{alt}
						width="2400"
						height="1600"
						decoding="async"
						class="block h-auto w-full dark:hidden"
					/>
					<img
						src={sources.dark}
						{alt}
						width="2400"
						height="1600"
						decoding="async"
						class="hidden h-auto w-full dark:block"
					/>
				</div>
			</DialogPrimitive.Content>
		</Dialog.Portal>
	</Dialog.Root>
	<figcaption class="mt-3 text-small text-muted-foreground">{caption}</figcaption>
</figure>
