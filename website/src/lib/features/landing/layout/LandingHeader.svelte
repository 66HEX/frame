<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { onDestroy, tick, untrack } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { Button } from '$lib/components/ui/button';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import { cn } from '$lib/utils';
	import { githubUrl } from '../data/content';
	import SiteBrand from '$lib/components/layout/SiteBrand.svelte';
	import HeaderMenuContent from './HeaderMenuContent.svelte';
	import MobileHeader from './MobileHeader.svelte';

	type MenuId = 'product' | 'resources';

	const desktop = new MediaQuery('(min-width: 64rem)', true);
	const menus: { value: MenuId; label: string }[] = [
		{ value: 'product', label: 'Product' },
		{ value: 'resources', label: 'Resources' }
	];
	const directLinks = [
		{ label: 'Documentation', href: '/docs' },
		{ label: 'GitHub', href: githubUrl }
	];

	const focusClass =
		'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring';
	const navAnchorClass = `inline-flex h-8 items-center justify-center rounded-md px-3 text-mini font-medium text-muted-foreground transition-[color,background-color] duration-100 ease-out hover:bg-foreground/8 hover:text-foreground ${focusClass}`;

	let activeMenu = $state<MenuId | ''>('');
	let exitingMenu = $state<MenuId | ''>('');
	let switchDirection = $state<'left' | 'right' | null>(null);
	let popupState = $state<'closed' | 'open' | 'closing'>('closed');
	let popupEntered = $state(false);
	let popupHeight = $state<number | null>(null);
	let popupTransitionsEnabled = $state(false);
	let scrollY = $state(0);

	let headerEl = $state<HTMLElement | null>(null);
	let popupEl = $state<HTMLDivElement | null>(null);
	let activePanelEl = $state<HTMLDivElement | null>(null);
	let triggerEls: Partial<Record<MenuId, HTMLButtonElement>> = {};
	const cachedHeights: Partial<Record<MenuId, number>> = {};

	let closeDelayTimer: ReturnType<typeof setTimeout> | undefined;
	let switchAnimTimer: ReturnType<typeof setTimeout> | undefined;
	let closeAnimTimer: ReturnType<typeof setTimeout> | undefined;
	let enterRaf: number | undefined;

	function prefersReducedMotion() {
		return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	}

	function clearTimers() {
		clearTimeout(closeDelayTimer);
		clearTimeout(switchAnimTimer);
		clearTimeout(closeAnimTimer);
		if (enterRaf !== undefined && typeof cancelAnimationFrame !== 'undefined') {
			cancelAnimationFrame(enterRaf);
		}
		closeDelayTimer = undefined;
		switchAnimTimer = undefined;
		closeAnimTimer = undefined;
		enterRaf = undefined;
	}

	function cancelCloseDelay() {
		clearTimeout(closeDelayTimer);
		closeDelayTimer = undefined;
	}

	$effect(() => {
		const menu = activeMenu;
		const state = popupState;
		const panel = activePanelEl;
		if (!menu || state === 'closed' || !panel) return;

		const nextHeight = panel.offsetHeight;
		cachedHeights[menu] = nextHeight;

		const wasUnmeasured = untrack(() => popupHeight === null);
		if (untrack(() => popupHeight) !== nextHeight) {
			popupHeight = nextHeight;
		}

		if (wasUnmeasured || !untrack(() => popupEntered)) {
			void tick().then(() => {
				if (!popupEl || popupState !== 'open') return;
				// Commit initial geometry and scale(0.98)/opacity(0) before enabling transitions
				void popupEl.offsetHeight;
				if (prefersReducedMotion()) {
					popupTransitionsEnabled = true;
					popupEntered = true;
				} else if (typeof requestAnimationFrame !== 'undefined') {
					if (enterRaf !== undefined) cancelAnimationFrame(enterRaf);
					enterRaf = requestAnimationFrame(() => {
						popupTransitionsEnabled = true;
						popupEntered = true;
						enterRaf = undefined;
					});
				}
			});
		}
	});

	function openMenu(value: MenuId) {
		cancelCloseDelay();
		clearTimeout(closeAnimTimer);
		closeAnimTimer = undefined;

		if (activeMenu === value && popupState === 'open') return;

		const wasOpen = popupState === 'open' || popupState === 'closing';
		const prevMenu = activeMenu;

		if (wasOpen && prevMenu && prevMenu !== value) {
			const prevIndex = menus.findIndex((m) => m.value === prevMenu);
			const nextIndex = menus.findIndex((m) => m.value === value);
			switchDirection = nextIndex > prevIndex ? 'right' : 'left';
			exitingMenu = prevMenu;
			clearTimeout(switchAnimTimer);
			if (prefersReducedMotion()) {
				exitingMenu = '';
			} else {
				switchAnimTimer = setTimeout(() => {
					exitingMenu = '';
				}, 220);
			}
		} else if (!wasOpen) {
			switchDirection = null;
			exitingMenu = '';
			popupEntered = false;
			popupTransitionsEnabled = false;
		}

		const cached = cachedHeights[value];
		if (cached !== undefined) {
			popupHeight = cached;
		}

		activeMenu = value;
		popupState = 'open';
		if (wasOpen) {
			popupEntered = true;
			popupTransitionsEnabled = true;
		}
	}

	function closeMenuImmediately() {
		clearTimers();
		activeMenu = '';
		exitingMenu = '';
		switchDirection = null;
		popupState = 'closed';
		popupEntered = false;
		popupHeight = null;
		popupTransitionsEnabled = false;
	}

	function closeMenu(options?: { restoreFocus?: boolean }) {
		cancelCloseDelay();
		if (popupState === 'closed') return;

		const closingTrigger = activeMenu ? triggerEls[activeMenu] : undefined;

		if (prefersReducedMotion()) {
			closeMenuImmediately();
			if (options?.restoreFocus) closingTrigger?.focus();
			return;
		}

		popupState = 'closing';
		popupEntered = false;
		clearTimeout(closeAnimTimer);
		closeAnimTimer = setTimeout(() => {
			closeMenuImmediately();
		}, 220);

		if (options?.restoreFocus) closingTrigger?.focus();
	}

	function scheduleClose() {
		cancelCloseDelay();
		closeDelayTimer = setTimeout(() => {
			closeMenu();
		}, 150);
	}

	function handlePopupTransitionEnd(event: TransitionEvent) {
		if (event.target !== popupEl || event.propertyName !== 'opacity') return;
		if (popupState === 'closing') {
			closeMenuImmediately();
		}
	}

	function handleExitingAnimationEnd(event: AnimationEvent) {
		if (event.currentTarget !== event.target) return;
		clearTimeout(switchAnimTimer);
		switchAnimTimer = undefined;
		exitingMenu = '';
	}

	function handleTriggerPointerEnter(event: PointerEvent, value: MenuId) {
		if (event.pointerType !== 'mouse') return;
		openMenu(value);
	}

	function handleTriggerClick(event: MouseEvent, value: MenuId) {
		// Clicking an already-open trigger via mouse keeps the menu open (Linear behavior);
		// keyboard activation (detail === 0) toggles it cleanly.
		if (activeMenu === value && popupState === 'open') {
			if (event.detail === 0) closeMenu();
			return;
		}
		openMenu(value);
	}

	function handleTriggerKeydown(event: KeyboardEvent, index: number) {
		if (event.key === 'ArrowRight') {
			event.preventDefault();
			const next = menus[(index + 1) % menus.length];
			triggerEls[next.value]?.focus();
			if (popupState === 'open') openMenu(next.value);
		} else if (event.key === 'ArrowLeft') {
			event.preventDefault();
			const prev = menus[(index - 1 + menus.length) % menus.length];
			triggerEls[prev.value]?.focus();
			if (popupState === 'open') openMenu(prev.value);
		} else if (event.key === 'ArrowDown' && popupState !== 'open') {
			event.preventDefault();
			openMenu(menus[index].value);
		}
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && popupState !== 'closed') {
			event.preventDefault();
			closeMenu({ restoreFocus: true });
		}
	}

	function handleWindowPointerDown(event: PointerEvent) {
		if (popupState === 'closed' || !headerEl) return;
		if (!headerEl.contains(event.target as Node)) {
			closeMenu();
		}
	}

	afterNavigate(closeMenuImmediately);
	onDestroy(clearTimers);
</script>

<svelte:window
	bind:scrollY
	onresize={closeMenuImmediately}
	onkeydown={handleWindowKeydown}
	onpointerdown={handleWindowPointerDown}
/>

<!-- Preserve the fixed header's measured 56px footprint in the page flow. -->
<div class="h-14">
	<header
		bind:this={headerEl}
		data-landing-header
		data-scrolled={scrollY > 10 ? '' : undefined}
		class="fixed inset-x-0 top-0 z-50 lg:h-14"
	>
		{#if desktop.current}
			<div class="linear-header-bar relative h-14">
				<div
					aria-hidden="true"
					class={cn(
						'pointer-events-none absolute inset-0 border-b transition-[background-color,border-color,backdrop-filter] duration-150 ease-out',
						scrollY > 10
							? 'border-border bg-background/80 backdrop-blur-[20px]'
							: 'border-transparent bg-transparent'
					)}
				></div>
				<div class="relative z-10 mx-auto flex h-full w-full max-w-6xl items-center justify-between px-6 sm:px-10">
					<div class="flex flex-1 justify-start items-center">
						<a href="/" aria-label="Frame home" class={`rounded-md flex items-center ${focusClass}`}>
							<SiteBrand />
						</a>
					</div>

					<nav aria-label="Primary navigation" class="flex items-center">
						<ul class="flex list-none items-center gap-0.5">
							{#each menus as menu, index (menu.value)}
								{@const isOpen = activeMenu === menu.value && popupState === 'open'}
								<li class="relative">
									<button
										bind:this={triggerEls[menu.value]}
										type="button"
										aria-expanded={isOpen}
										aria-controls="desktop-nav-popup"
										data-popup-open={isOpen ? '' : undefined}
										class={cn(
											'linear-nav-trigger relative',
											navAnchorClass,
											isOpen && 'bg-foreground/8 text-foreground'
										)}
										onpointerenter={(event) => handleTriggerPointerEnter(event, menu.value)}
										onpointerleave={(event) => {
											if (event.pointerType === 'mouse') scheduleClose();
										}}
										onclick={(event) => handleTriggerClick(event, menu.value)}
										onkeydown={(event) => handleTriggerKeydown(event, index)}
									>
										{menu.label}
									</button>
								</li>
							{/each}
							{#each directLinks as item (item.href)}
								<li>
									<a
										href={item.href}
										class={navAnchorClass}
										onpointerenter={(event) => {
											if (event.pointerType === 'mouse') scheduleClose();
										}}
									>
										{item.label}
									</a>
								</li>
							{/each}
						</ul>

						<div class="mx-3 h-4 w-px bg-border" aria-hidden="true"></div>

						<div class="flex items-center gap-2">
							<Button href="/#download" variant="default" size="sm">Download</Button>
							<ThemeToggle />
						</div>
					</nav>
				</div>
			</div>
			{#if popupState !== 'closed' && activeMenu}
				<div
					class="pointer-events-none absolute top-[calc(100%-4px)] right-0 left-0 mx-auto flex max-w-6xl justify-end px-6 sm:px-10"
				>
					<div
						bind:this={popupEl}
						id="desktop-nav-popup"
						role="region"
						aria-label={activeMenu === 'product' ? 'Product navigation' : 'Resources navigation'}
						data-open={popupEntered ? '' : undefined}
						data-ending-style={popupState === 'closing' ? '' : undefined}
						class={cn(
							'linear-nav-popup pointer-events-auto relative max-w-full overflow-hidden rounded-xl bg-muted text-card-foreground shadow-sm',
							popupTransitionsEnabled && 'linear-nav-popup-transitions'
						)}
						style:height={popupHeight !== null ? `${popupHeight}px` : undefined}
						ontransitionend={handlePopupTransitionEnd}
						onpointerenter={cancelCloseDelay}
						onpointerleave={(event) => {
							if (event.pointerType === 'mouse') scheduleClose();
						}}
					>
						<div class="relative w-full">
							{#if exitingMenu}
								{#key `exit-${exitingMenu}`}
									<div
										aria-hidden="true"
										inert
										data-ending-style=""
										data-activation-direction={switchDirection ?? undefined}
										class="linear-nav-content pointer-events-none absolute inset-x-0 top-0"
										onanimationend={handleExitingAnimationEnd}
									>
										<HeaderMenuContent menu={exitingMenu} />
									</div>
								{/key}
							{/if}
							{#key `active-${activeMenu}`}
								<div
									bind:this={activePanelEl}
									data-open=""
									data-activation-direction={switchDirection ?? undefined}
									class="linear-nav-content relative"
								>
									<HeaderMenuContent menu={activeMenu} onNavigate={closeMenuImmediately} />
								</div>
							{/key}
						</div>
					</div>
				</div>
			{/if}
		{:else}
			<MobileHeader scrolled={scrollY > 10} />
		{/if}
	</header>
</div>

<style>
	/* Invisible hover bridges matching Linear's trigger hit areas */
	.linear-nav-trigger::before {
		content: '';
		position: absolute;
		inset: 0 -8px;
		z-index: 0;
	}

	.linear-nav-trigger[data-popup-open]::after {
		content: '';
		position: absolute;
		top: 100%;
		left: -32px;
		right: -32px;
		height: 32px;
	}

	.linear-nav-popup {
		transform-origin: top;
		opacity: 0;
		transform: scale(0.98);
	}

	.linear-nav-popup[data-open] {
		opacity: 1;
		transform: none;
	}

	.linear-nav-popup-transitions {
		transition:
			opacity 180ms cubic-bezier(0.25, 0.46, 0.45, 0.94),
			transform 180ms cubic-bezier(0.25, 0.46, 0.45, 0.94),
			height 220ms cubic-bezier(0.25, 0.46, 0.45, 0.94);
	}

	@media (prefers-reduced-motion: no-preference) {
		.linear-nav-content[data-activation-direction] {
			--duration: 180ms;
			--anim-amount: 48px;
			animation: var(--duration) cubic-bezier(0.455, 0.03, 0.515, 0.955) both;
		}

		.linear-nav-content[data-activation-direction]
			:global(:is(.linear-sub-items, .linear-content-footer) > *) {
			animation: var(--duration) cubic-bezier(0.455, 0.03, 0.515, 0.955) both;
			will-change: transform;
		}

		.linear-nav-content[data-open][data-activation-direction='left'] {
			animation-name: linear-fade-in;
		}

		.linear-nav-content[data-open][data-activation-direction='left']
			:global(:is(.linear-sub-items, .linear-content-footer) > *) {
			animation-name: linear-slide-from-left;
		}

		.linear-nav-content[data-open][data-activation-direction='right'] {
			animation-name: linear-fade-in;
		}

		.linear-nav-content[data-open][data-activation-direction='right']
			:global(:is(.linear-sub-items, .linear-content-footer) > *) {
			animation-name: linear-slide-from-right;
		}

		.linear-nav-content[data-ending-style][data-activation-direction='right'] {
			animation-name: linear-fade-out;
		}

		.linear-nav-content[data-ending-style][data-activation-direction='right']
			:global(:is(.linear-sub-items, .linear-content-footer) > *) {
			animation-name: linear-slide-to-left;
		}

		.linear-nav-content[data-ending-style][data-activation-direction='left'] {
			animation-name: linear-fade-out;
		}

		.linear-nav-content[data-ending-style][data-activation-direction='left']
			:global(:is(.linear-sub-items, .linear-content-footer) > *) {
			animation-name: linear-slide-to-right;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.linear-nav-popup-transitions {
			transition: none;
		}
	}

	@keyframes linear-fade-in {
		0% {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes linear-fade-out {
		0% {
			opacity: 1;
		}
		to {
			opacity: 0;
		}
	}

	@keyframes linear-slide-from-right {
		0% {
			transform: translateX(var(--anim-amount));
		}
		to {
			transform: translateX(0);
		}
	}

	@keyframes linear-slide-from-left {
		0% {
			transform: translateX(calc(-1 * var(--anim-amount)));
		}
		to {
			transform: translateX(0);
		}
	}

	@keyframes linear-slide-to-right {
		0% {
			transform: translateX(0);
		}
		to {
			transform: translateX(var(--anim-amount));
		}
	}

	@keyframes linear-slide-to-left {
		0% {
			transform: translateX(0);
		}
		to {
			transform: translateX(calc(-1 * var(--anim-amount)));
		}
	}
</style>
