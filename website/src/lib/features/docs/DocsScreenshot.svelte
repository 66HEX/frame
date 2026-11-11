<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import { Dialog as DialogPrimitive } from 'bits-ui';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as Card from '$lib/components/ui/card';
	import { XmarkIcon } from '$lib/icons';

	type Props = { scene: string; alt: string; caption: string };
	let { scene, alt, caption }: Props = $props();
	let open = $state(false);
	let moving = $state(false);
	let trigger = $state<HTMLButtonElement | null>(null);
	let viewport = $state<HTMLDivElement | null>(null);
	let overlay = $state<HTMLElement | null>(null);
	let content = $state<HTMLElement | null>(null);
	let dialogOpacity = $state(0);
	let requestedOpen = false;
	let operation = 0;
	let floatingImage: HTMLImageElement | null = null;
	let animation: Animation | null = null;
	let fadeAnimations: Animation[] = [];
	const sources = $derived({
		light: `/media/docs/${scene}-light.webp`,
		dark: `/media/docs/${scene}-dark.webp`
	});

	function visibleImage(container: HTMLElement | null) {
		return [...(container?.querySelectorAll('img') ?? [])].find(
			(image) => getComputedStyle(image).display !== 'none'
		);
	}

	function cancelAnimations() {
		animation?.cancel();
		animation = null;
		for (const fade of fadeAnimations) fade.cancel();
		fadeAnimations = [];
	}

	function removeAnimation() {
		cancelAnimations();
		floatingImage?.remove();
		floatingImage = null;
	}

	function settleImmediately() {
		operation += 1;
		removeAnimation();
		moving = false;
		dialogOpacity = requestedOpen ? 1 : 0;
		open = requestedOpen;
	}

	function changeOpen(value: boolean) {
		if (value === requestedOpen) return;
		requestedOpen = value;
		void moveTo(value);
	}

	async function moveTo(nextOpen: boolean) {
		const currentOperation = ++operation;
		// Read the visible in-flight position before cancellation so an interrupted
		// opening returns smoothly from that point instead of jumping to full size.
		const interruptedRect = floatingImage?.getBoundingClientRect();
		const fromOpacity = overlay ? Number(getComputedStyle(overlay).opacity) : 0;
		// Freeze the visible layers before cancelling their animations. Decoding
		// or the next Svelte render must not reveal the previous destination.
		if (floatingImage && interruptedRect) {
			Object.assign(floatingImage.style, {
				left: `${interruptedRect.left}px`, top: `${interruptedRect.top}px`,
				width: `${interruptedRect.width}px`, height: `${interruptedRect.height}px`,
				transform: 'none'
			});
		}
		if (overlay) overlay.style.opacity = `${fromOpacity}`;
		if (content) content.style.opacity = `${fromOpacity}`;
		dialogOpacity = fromOpacity;
		cancelAnimations();

		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			settleImmediately();
			return;
		}

		const source = visibleImage(open ? viewport : trigger);
		const from = interruptedRect ?? source?.getBoundingClientRect();
		if (!source || !from?.width || !from.height || typeof source.animate !== 'function') {
			settleImmediately();
			return;
		}

		const image = floatingImage ?? new Image();
		floatingImage = image;
		image.src = source.currentSrc || source.src;
		image.alt = '';
		image.setAttribute('aria-hidden', 'true');
		image.dataset.screenshotFlip = scene;
		// A decoded copy avoids flashing an empty layer on a cold image cache.
		try {
			await image.decode();
		} catch {
			if (currentOperation === operation) settleImmediately();
			return;
		}
		if (currentOperation !== operation) return;

		if (nextOpen) {
			moving = true;
			open = true;
			await tick();
			if (currentOperation !== operation) return;
		}

		const destination = visibleImage(nextOpen ? viewport : trigger);
		const to = destination?.getBoundingClientRect();
		if (!to?.width || !to.height) {
			settleImmediately();
			return;
		}

		Object.assign(image.style, {
			position: 'fixed',
			left: `${to.left}px`,
			top: `${to.top}px`,
			width: `${to.width}px`,
			height: `${to.height}px`,
			maxWidth: 'none',
			margin: '0',
			objectFit: 'contain',
			transformOrigin: 'top left',
			transform: 'none',
			borderRadius: 'var(--radius-xl)',
			boxShadow: 'var(--shadow-lg)',
			pointerEvents: 'none',
			zIndex: '90'
		});
		document.body.append(image);
		moving = true;

		try {
			const timing = { duration: nextOpen ? 300 : 240, easing: 'cubic-bezier(0.22, 1, 0.36, 1)', fill: 'both' as const };
			animation = image.animate(
				[
					{ transform: `translate(${from.left - to.left}px, ${from.top - to.top}px) scale(${from.width / to.width}, ${from.height / to.height})` },
					{ transform: 'translate(0, 0) scale(1, 1)' }
				],
				timing
			);
			const fade = [{ opacity: fromOpacity }, { opacity: nextOpen ? 1 : 0 }];
			for (const layer of [overlay, content]) {
				if (layer) fadeAnimations.push(layer.animate(fade, timing));
			}
			await Promise.all([animation.finished, ...fadeAnimations.map((fade) => fade.finished)]);
		} catch {
			// Cancellation is expected when Escape, another request, or a resize
			// interrupts the movement. The latest operation owns the image layer.
		}
		if (currentOperation !== operation) return;

		open = nextOpen;
		moving = false;
		dialogOpacity = nextOpen ? 1 : 0;
		await tick();
		if (currentOperation !== operation) return;
		removeAnimation();
	}

	onMount(() => {
		const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
		const finishMovement = () => {
			if (floatingImage) settleImmediately();
		};
		window.addEventListener('resize', finishMovement);
		reducedMotion.addEventListener('change', finishMovement);
		return () => {
			window.removeEventListener('resize', finishMovement);
			reducedMotion.removeEventListener('change', finishMovement);
		};
	});

	onDestroy(() => {
		operation += 1;
		removeAnimation();
	});
</script>

<figure class="my-8 min-w-0" data-doc-screenshot={scene}>
	<Dialog.Root bind:open={() => open, changeOpen}>
		<Card.Root size="sm" class="gap-0 bg-muted p-1">
			<Dialog.Trigger>
				{#snippet child({ props })}
					<button
						{...props}
						bind:this={trigger}
						type="button"
						aria-label={`View full size: ${caption}`}
						class="group relative block w-full cursor-zoom-in overflow-hidden rounded-lg bg-card text-start shadow-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
					>
						<img src={sources.light} {alt} width="2400" height="1600" loading="lazy" decoding="async" class="block h-auto w-full transition-none dark:hidden" class:invisible={open || moving} />
						<img src={sources.dark} {alt} width="2400" height="1600" loading="lazy" decoding="async" class="hidden h-auto w-full transition-none dark:block" class:invisible={open || moving} />
					</button>
				{/snippet}
			</Dialog.Trigger>
		</Card.Root>

		<Dialog.Portal>
			<Dialog.Overlay bind:ref={overlay} style={`opacity: ${dialogOpacity}`} onclick={() => changeOpen(false)} class="bg-black/80 supports-backdrop-filter:backdrop-blur-none data-open:animate-none data-closed:animate-none" />
			<DialogPrimitive.Content
				bind:ref={content}
				data-screenshot-dialog={scene}
				data-moving={moving}
				class="pointer-events-none! fixed inset-0 z-[100] grid place-items-center bg-transparent px-3 pt-[var(--screenshot-top)] pb-3 outline-none"
				style={`--screenshot-top: max(4.5rem, calc(env(safe-area-inset-top) + 3.5rem)); opacity: ${dialogOpacity}`}
			>
				<Dialog.Title class="sr-only">{caption}</Dialog.Title>
				<Dialog.Description class="sr-only">{alt} Press Escape or click outside the image to close.</Dialog.Description>
				<button
					type="button"
					class="pointer-events-auto fixed top-[max(1rem,env(safe-area-inset-top))] right-[max(1rem,env(safe-area-inset-right))] z-10 flex size-11 items-center justify-center rounded-md text-white opacity-80 outline-none transition-opacity hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-white/70"
					aria-label="Close screenshot"
					onclick={() => changeOpen(false)}
				>
					<XmarkIcon class="size-5" aria-hidden="true" />
				</button>
				<div
					bind:this={viewport}
					class="pointer-events-auto aspect-[3/2] w-[min(calc(100vw-1.5rem),calc((100svh-var(--screenshot-top)-0.75rem)*1.5))] overflow-hidden rounded-xl shadow-lg"
					style:opacity={moving ? 0 : 1}
				>
					<img src={sources.light} {alt} width="2400" height="1600" decoding="async" class="block h-auto w-full dark:hidden" />
					<img src={sources.dark} {alt} width="2400" height="1600" decoding="async" class="hidden h-auto w-full dark:block" />
				</div>
			</DialogPrimitive.Content>
		</Dialog.Portal>
	</Dialog.Root>
	<figcaption class="mt-3 text-sm leading-6 text-muted-foreground">{caption}</figcaption>
</figure>
