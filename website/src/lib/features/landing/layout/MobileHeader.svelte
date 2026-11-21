<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import { Accordion } from 'bits-ui';
	import { Button } from '$lib/components/ui/button';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import { ChevronDownIcon, Menu2Icon, XmarkIcon } from '$lib/icons';
	import { cn } from '$lib/utils';
	import { githubUrl } from '../data/content';
	import HeaderMenuContent from './HeaderMenuContent.svelte';
	import SiteBrand from '$lib/components/layout/SiteBrand.svelte';

	let { scrolled = false }: { scrolled?: boolean } = $props();

	let open = $state(false);
	let isTransitioning = $state(false);
	let expandedGroup = $state('');
	let scrollbarCompensation = $state(0);
	let headerContainerEl = $state<HTMLDivElement | null>(null);
	let toggleButtonEl = $state<HTMLButtonElement | null>(null);

	let savedScrollLock: { overflow: string; paddingRight: string } | null = null;
	let transitionTimer: ReturnType<typeof setTimeout> | undefined;

	const menuPanelId = 'mobile-navigation-panel';
	const menus = [
		{ value: 'product', label: 'Product' },
		{ value: 'resources', label: 'Resources' }
	] as const;
	const focusClass = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring';
	const iconButtonClass = `-mr-3 flex size-11 items-center justify-center rounded-lg text-foreground ${focusClass}`;

	function clearTransitionTimer() {
		clearTimeout(transitionTimer);
		transitionTimer = undefined;
	}

	function prefersReducedMotion() {
		return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	}

	function lockScroll() {
		if (typeof document === 'undefined' || savedScrollLock) return;
		const root = document.documentElement;
		const scrollbarWidth = window.innerWidth - root.clientWidth;
		savedScrollLock = {
			overflow: root.style.overflow,
			paddingRight: root.style.paddingRight
		};
		if (scrollbarWidth > 0) {
			const currentPadding = Number.parseFloat(getComputedStyle(root).paddingRight) || 0;
			root.style.paddingRight = `${currentPadding + scrollbarWidth}px`;
			scrollbarCompensation = scrollbarWidth;
		}
		root.style.overflow = 'hidden';
	}

	function unlockScroll() {
		if (typeof document === 'undefined' || !savedScrollLock) return;
		const root = document.documentElement;
		root.style.overflow = savedScrollLock.overflow;
		root.style.paddingRight = savedScrollLock.paddingRight;
		savedScrollLock = null;
		scrollbarCompensation = 0;
	}

	function openMenu() {
		if (open) return;
		clearTransitionTimer();
		lockScroll();
		open = true;
		if (!prefersReducedMotion()) {
			isTransitioning = true;
			transitionTimer = setTimeout(() => {
				isTransitioning = false;
			}, 220);
		}
	}

	function closeMenu(options?: { restoreFocus?: boolean }) {
		if (!open) {
			expandedGroup = '';
			return;
		}
		clearTransitionTimer();
		open = false;
		unlockScroll();
		if (prefersReducedMotion()) {
			isTransitioning = false;
			expandedGroup = '';
		} else {
			isTransitioning = true;
			transitionTimer = setTimeout(() => {
				isTransitioning = false;
				expandedGroup = '';
			}, 220);
		}
		if (options?.restoreFocus) toggleButtonEl?.focus();
	}

	function toggleMenu() {
		if (open) closeMenu();
		else openMenu();
	}

	function handleTransitionEnd(event: TransitionEvent) {
		if (event.target !== headerContainerEl || event.propertyName !== 'height') return;
		clearTransitionTimer();
		isTransitioning = false;
		if (!open) expandedGroup = '';
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (!open) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			closeMenu({ restoreFocus: true });
			return;
		}
		if (event.key !== 'Tab' || !headerContainerEl) return;
		const focusables = Array.from(
			headerContainerEl.querySelectorAll<HTMLElement>(
				'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])'
			)
		).filter(
			(el) =>
				!el.closest('[inert]') &&
				el.getClientRects().length > 0 &&
				getComputedStyle(el).visibility !== 'hidden'
		);
		if (focusables.length === 0) return;
		const first = focusables[0];
		const last = focusables[focusables.length - 1];
		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	}

	afterNavigate(() => closeMenu());
	onDestroy(() => {
		clearTransitionTimer();
		unlockScroll();
	});
</script>

<svelte:window
	onkeydown={handleWindowKeydown}
	onresize={() => {
		if (window.matchMedia('(min-width: 64rem)').matches) closeMenu();
	}}
/>

<div
	bind:this={headerContainerEl}
	data-frame-mobile-header
	data-state={open ? 'open' : 'closed'}
	class={cn('mobile-header relative overflow-hidden lg:hidden', open ? 'h-dvh' : 'h-14')}
	style:padding-right={scrollbarCompensation ? `${scrollbarCompensation}px` : undefined}
	ontransitionend={handleTransitionEnd}
>
	<div
		aria-hidden="true"
		class={cn(
			'mobile-header-surface pointer-events-none absolute inset-0 bg-background/75 backdrop-blur',
			open || scrolled ? 'opacity-100' : 'opacity-0'
		)}
	></div>
	<div class="relative z-10 flex h-full flex-col">
		<div class="h-14 shrink-0 border-b border-border">
			<div class="mx-auto flex h-full w-full max-w-6xl items-center justify-between px-6 md:px-10">
				<a
					href="/"
					aria-label="Frame home"
					class={`rounded-sm ${focusClass}`}
					onclick={() => closeMenu()}
				>
					<SiteBrand />
				</a>
				<button
					bind:this={toggleButtonEl}
					type="button"
					aria-expanded={open}
					aria-controls={menuPanelId}
					aria-label={open ? 'Close menu' : 'Open menu'}
					class={iconButtonClass}
					onclick={toggleMenu}
				>
					<span class="relative flex size-5 items-center justify-center" aria-hidden="true">
						<Menu2Icon
							class={cn(
								'absolute inset-0 size-5 transition-[opacity,transform] duration-200 ease-out',
								open && 'scale-75 rotate-45 opacity-0'
							)}
						/>
						<XmarkIcon
							class={cn(
								'absolute inset-0 size-5 transition-[opacity,transform] duration-200 ease-out',
								!open && 'scale-75 -rotate-45 opacity-0'
							)}
						/>
					</span>
				</button>
			</div>
		</div>
		<div
			id={menuPanelId}
			aria-hidden={!open}
			inert={!open}
			class={cn(
				'mobile-header-panel min-h-0 flex-1 overscroll-contain',
				open && !isTransitioning ? 'overflow-y-auto' : 'overflow-y-hidden'
			)}
		>
			<div class="mx-auto w-full max-w-6xl px-6 pb-8 md:px-10">
				<nav aria-label="Mobile navigation" class="-mx-4">
					<Accordion.Root type="single" bind:value={expandedGroup}>
						{#each menus as menu (menu.value)}
							<Accordion.Item
								value={menu.value}
								class="relative after:pointer-events-none after:absolute after:inset-x-4 after:bottom-0 after:h-px after:bg-border"
							>
								<Accordion.Header level={3}>
									<Accordion.Trigger
										class={`group flex min-h-14 w-full items-center justify-between rounded-md px-4 py-3 text-left text-title-1 font-normal data-[state=open]:bg-foreground/5 ${focusClass}`}
									>
										{menu.label}
										<ChevronDownIcon
											class="size-4 shrink-0 text-muted-foreground group-data-[state=open]:rotate-180 motion-safe:transition-transform motion-safe:duration-200"
											aria-hidden="true"
										/>
									</Accordion.Trigger>
								</Accordion.Header>
								<Accordion.Content data-frame-mobile-group class="overflow-hidden">
									<HeaderMenuContent menu={menu.value} mobile onNavigate={() => closeMenu()} />
								</Accordion.Content>
							</Accordion.Item>
						{/each}
					</Accordion.Root>
					<div class="mx-4 border-b border-border">
						<a
							href="/docs"
							class={`flex min-h-15 items-center rounded-lg py-3 text-title-1 ${focusClass}`}
							onclick={() => closeMenu()}>Documentation</a
						>
					</div>
					<div class="mx-4 border-b border-border">
						<a
							href={githubUrl}
							class={`flex min-h-15 items-center rounded-lg py-3 text-title-1 ${focusClass}`}
							onclick={() => closeMenu()}>GitHub</a
						>
					</div>
				</nav>
				<div class="mt-6 flex items-center gap-3">
					<Button href="/#download" variant="outline" class="flex-1" onclick={() => closeMenu()}
						>Download Frame</Button
					>
					<ThemeToggle showTooltip={false} />
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.mobile-header {
		transition: height 200ms cubic-bezier(0.4, 0, 0.2, 1);
	}

	.mobile-header-surface {
		transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1);
	}

	.mobile-header-panel {
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 160ms ease-out,
			visibility 0s linear 200ms;
	}

	.mobile-header[data-state='open'] .mobile-header-panel {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 180ms ease-out 20ms,
			visibility 0s linear 0s;
	}

	@media (prefers-reduced-motion: no-preference) {
		:global([data-frame-mobile-group][data-state='open']) {
			animation: group-expand 200ms ease-out;
		}

		:global([data-frame-mobile-group][data-state='closed']) {
			animation: group-collapse 200ms ease-out;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.mobile-header,
		.mobile-header-surface,
		.mobile-header-panel,
		.mobile-header[data-state='open'] .mobile-header-panel {
			transition: none;
		}
	}

	@keyframes group-expand {
		from {
			height: 0;
		}
		to {
			height: var(--bits-accordion-content-height);
		}
	}

	@keyframes group-collapse {
		from {
			height: var(--bits-accordion-content-height);
		}
		to {
			height: 0;
		}
	}
</style>
