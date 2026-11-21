import { tick } from 'svelte';
import { flipKeyframes, radius, readSurface, scaleShadow } from './media-transition';

type Elements = {
	thumbnail: HTMLElement | undefined;
	viewport: HTMLElement | undefined;
	overlay: HTMLElement | null;
	content: HTMLElement | null;
	trigger: HTMLButtonElement | null;
};

type Options = {
	elements: () => Elements;
	frame: (signal: AbortSignal) => HTMLElement | null | Promise<HTMLElement | null>;
	onRequest?: () => void;
	onMounted?: (open: boolean, frame: HTMLElement | null, signal: AbortSignal) => void;
	onSettled?: () => void;
};

/** Owns only the transition. Media readiness must never hold the dialog open. */
export class MediaLightbox {
	open = $state(false);
	moving = $state(false);
	opacity = $state(0);
	requestedOpen = false;
	#options: Options;
	#operation = 0;
	#controller: AbortController | null = null;
	#floating: HTMLElement | null = null;
	#animations: Animation[] = [];
	#disposed = false;
	#transitioning = false;

	constructor(options: Options) {
		this.#options = options;
	}

	changeOpen(value: boolean) {
		if (this.#disposed || value === this.requestedOpen) return;
		this.requestedOpen = value;
		void this.#moveTo(value);
	}

	#cancelAnimations() {
		for (const animation of this.#animations) animation.cancel();
		this.#animations = [];
	}

	#removeFrame() {
		this.#cancelAnimations();
		this.#floating?.remove();
		this.#floating = null;
	}

	async #moveTo(nextOpen: boolean, immediate = false) {
		const operation = ++this.#operation;
		this.#controller?.abort();
		const controller = new AbortController();
		this.#controller = controller;
		const current = () => !this.#disposed && operation === this.#operation;
		const elements = this.#options.elements();
		const source = this.#floating ?? (this.open ? elements.viewport : elements.thumbnail);
		const from = source ? readSurface(source, source === this.#floating) : null;
		const opacity = elements.overlay ? Number(getComputedStyle(elements.overlay).opacity) : 0;

		// Bake the rendered geometry before cancelling WAAPI's fill. The next
		// operation starts at this exact frame, including its corners and shadow.
		if (this.#floating && from) {
			Object.assign(this.#floating.style, {
				left: `${from.rect.left}px`,
				top: `${from.rect.top}px`,
				width: `${from.rect.width}px`,
				height: `${from.rect.height}px`,
				transform: 'none',
				borderRadius: radius(from.corners),
				boxShadow: from.shadow
			});
		}
		for (const layer of [elements.overlay, elements.content]) {
			if (layer) layer.style.opacity = `${opacity}`;
		}
		this.opacity = opacity;
		this.#cancelAnimations();
		this.#options.onRequest?.();
		this.#transitioning = true;

		let frame = this.#floating;
		const animate =
			!immediate &&
			(nextOpen || this.open) &&
			from?.rect.width &&
			from.rect.height &&
			!window.matchMedia('(prefers-reduced-motion: reduce)').matches &&
			typeof source?.animate === 'function';
		if (animate && !frame) {
			try {
				frame = await this.#options.frame(controller.signal);
			} catch {
				// A failed snapshot/decode falls back to the live media element.
				frame = null;
			}
			if (!current()) {
				frame?.remove();
				return;
			}
			this.#floating = frame;
		}

		if (nextOpen) this.open = true;
		this.moving = !!(animate && frame);
		await tick();
		if (!current()) return;
		this.#options.onMounted?.(nextOpen, frame, controller.signal);
		const destination = nextOpen
			? this.#options.elements().viewport
			: this.#options.elements().thumbnail;
		const to = destination ? readSurface(destination) : null;

		if (animate && frame && from && to?.rect.width && to.rect.height) {
			Object.assign(frame.style, {
				position: 'fixed',
				left: `${to.rect.left}px`,
				top: `${to.rect.top}px`,
				width: `${to.rect.width}px`,
				height: `${to.rect.height}px`,
				maxWidth: 'none',
				margin: '0',
				objectFit: 'contain',
				transformOrigin: 'top left',
				transform: 'none',
				borderRadius: radius(to.corners),
				boxShadow: to.shadow,
				pointerEvents: 'none',
				zIndex: '90'
			});
			document.body.append(frame);
			const timing: KeyframeAnimationOptions = {
				duration: nextOpen ? 300 : 240,
				easing: 'cubic-bezier(0.22, 1, 0.36, 1)',
				fill: 'both'
			};
			try {
				this.#animations.push(frame.animate(flipKeyframes(from, to), timing));
				this.#animations.push(
					frame.animate(
						[
							{
								boxShadow: scaleShadow(
									from.shadow,
									to.rect.width / from.rect.width,
									to.rect.height / from.rect.height
								)
							},
							{ boxShadow: to.shadow }
						],
						timing
					)
				);
				for (const layer of [this.#options.elements().overlay, this.#options.elements().content]) {
					if (layer) {
						this.#animations.push(layer.animate([{ opacity }, { opacity: nextOpen ? 1 : 0 }], timing));
					}
				}
				await Promise.allSettled(this.#animations.map((animation) => animation.finished));
			} catch {
				// Unsupported animation details must not break opening or closing.
			}
		}
		if (!current()) return;
		this.open = nextOpen;
		this.moving = false;
		this.opacity = nextOpen ? 1 : 0;
		await tick();
		if (!current()) return;
		this.#removeFrame();
		this.#transitioning = false;
		// Restore after Svelte has removed the focus scope and scroll lock. This
		// also covers cancelling image preparation before a dialog ever mounts.
		const trigger = this.#options.elements().trigger;
		if (!nextOpen && trigger?.isConnected) trigger.focus({ preventScroll: true });
		this.#options.onSettled?.();
	}

	connect() {
		const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
		const finish = () => {
			if (this.#transitioning) void this.#moveTo(this.requestedOpen, true);
		};
		const cancelPreparation = (event: KeyboardEvent) => {
			// Bits UI takes over Escape once the dialog is mounted.
			if (event.key === 'Escape' && !event.defaultPrevented && this.requestedOpen && !this.open) {
				event.preventDefault();
				this.changeOpen(false);
			}
		};
		window.addEventListener('resize', finish);
		window.visualViewport?.addEventListener('resize', finish);
		reducedMotion.addEventListener('change', finish);
		document.addEventListener('keydown', cancelPreparation);
		return () => {
			window.removeEventListener('resize', finish);
			window.visualViewport?.removeEventListener('resize', finish);
			reducedMotion.removeEventListener('change', finish);
			document.removeEventListener('keydown', cancelPreparation);
		};
	}

	destroy() {
		this.#disposed = true;
		this.#operation += 1;
		this.#controller?.abort();
		this.#removeFrame();
	}
}
