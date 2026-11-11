<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { NavigationMenu } from 'bits-ui';
	import { Button } from '$lib/components/ui/button';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import { ChevronDownIcon } from '$lib/icons';
	import { cn } from '$lib/utils';
	import { githubUrl } from '../data/content';
	import SiteBrand from '$lib/components/layout/SiteBrand.svelte';
	import HeaderMenuContent from './HeaderMenuContent.svelte';
	import MobileHeader from './MobileHeader.svelte';

	const desktop = new MediaQuery('(min-width: 64rem)', true);
	const menus = [
		{ value: 'product', label: 'Product' },
		{ value: 'resources', label: 'Resources' }
	] as const;
	const directLinks = [
		{ label: 'Documentation', href: '/docs' },
		{ label: 'GitHub', href: githubUrl }
	];
	const triggerClass =
		'group inline-flex h-8 items-center justify-center rounded-md px-4 py-1 text-sm font-medium text-muted-foreground transition-[color,background-color,box-shadow] duration-150 hover:bg-foreground/5 hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring data-[state=open]:bg-foreground/5 data-[state=open]:text-foreground';

	let activeMenu = $state('');
	let scrollY = $state(0);
	let hoverTimer: ReturnType<typeof setTimeout> | undefined;
	let skipOpenDelayUntil = 0;
	// Measured from Libre: the surface height is independent of the exiting viewport.
	const menuHeights = { product: 280, resources: 220 } as const;
	const surfaceHeight = $derived(
		activeMenu in menuHeights
			? `calc(3.4rem + ${menuHeights[activeMenu as keyof typeof menuHeights]}px)`
			: '3.5rem'
	);

	function cancelHoverTimer() {
		clearTimeout(hoverTimer);
		hoverTimer = undefined;
	}

	function setActiveMenu(value: string) {
		cancelHoverTimer();
		if (activeMenu && !value) skipOpenDelayUntil = Date.now() + 300;
		activeMenu = value;
	}

	function closeNavigation() {
		setActiveMenu('');
	}

	// Bits adds 150ms even while open. Libre switches immediately, retaining
	// only the initial 200ms hover delay and 150ms grace when leaving the menu.
	function enterTrigger(event: PointerEvent, value: string) {
		if (event.pointerType !== 'mouse') return;
		cancelHoverTimer();
		if (activeMenu === value) return;
		const trigger = event.currentTarget as HTMLButtonElement;
		const open = () => {
			// Use the primitive's selection path to preserve direction and focus state.
			if (activeMenu !== value) trigger.click();
		};
		if (activeMenu || Date.now() < skipOpenDelayUntil) open();
		else hoverTimer = setTimeout(open, 200);
	}

	function leaveNavigation(event: PointerEvent) {
		if (event.pointerType !== 'mouse') return;
		cancelHoverTimer();
		hoverTimer = setTimeout(closeNavigation, 150);
	}

	afterNavigate(closeNavigation);
	onDestroy(cancelHoverTimer);
</script>

<svelte:window bind:scrollY onresize={closeNavigation} />

<!-- Preserve the fixed header's measured 56px footprint in the page flow. -->
<div class="h-14">
	{#if desktop.current && activeMenu}
		<div aria-hidden="true" onpointerdown={closeNavigation} class="frame-nav-backdrop fixed inset-0 z-40 bg-background/50 backdrop-blur"></div>
	{/if}
	<header
		data-landing-header
		data-scrolled={scrollY > 50 ? '' : undefined}
		class="fixed inset-x-0 top-0 z-50 h-14"
	>
		{#if desktop.current}
			<div
				aria-hidden="true"
				class={cn('header-surface absolute inset-x-0 top-0 border-b', activeMenu ? 'border-transparent bg-card/75 shadow-lg backdrop-blur' : scrollY > 50 ? 'border-border bg-background/75 backdrop-blur' : 'border-border bg-transparent')}
				style:height={surfaceHeight}
			></div>
			<NavigationMenu.Root class="relative" bind:value={() => activeMenu, setActiveMenu} onpointerleave={leaveNavigation} aria-label="Primary navigation">
				<div class="h-14">
					<div class="relative mx-auto flex h-14 max-w-6xl items-center justify-between px-6 sm:px-10">
						<a href="/" aria-label="Frame home"><SiteBrand /></a>
						<div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
							<NavigationMenu.List class="flex list-none items-center gap-3">
								{#each menus as menu (menu.value)}
									<NavigationMenu.Item value={menu.value} openOnHover={false}>
										<NavigationMenu.Trigger class={triggerClass} onpointerenter={(event) => enterTrigger(event, menu.value)} onpointerleave={leaveNavigation}>
											{menu.label}
											<ChevronDownIcon class="relative top-px ml-1.5 size-3 opacity-75 transition-transform duration-150 group-data-[state=open]:translate-y-px" aria-hidden="true" />
										</NavigationMenu.Trigger>
										<NavigationMenu.Content inert={activeMenu !== menu.value} class="frame-nav-content absolute top-0 left-0">
											<HeaderMenuContent menu={menu.value} />
										</NavigationMenu.Content>
									</NavigationMenu.Item>
								{/each}
								{#each directLinks as item (item.href)}
									<NavigationMenu.Item>
										<NavigationMenu.Link href={item.href} class={triggerClass} onpointerenter={leaveNavigation}>{item.label}</NavigationMenu.Link>
									</NavigationMenu.Item>
								{/each}
							</NavigationMenu.List>
						</div>
						<div class="flex items-center gap-2">
							<Button href="/#download" variant="outline">Download</Button>
							<ThemeToggle />
						</div>
						{#if activeMenu}<div class="header-divider absolute inset-x-6 bottom-0" aria-hidden="true"></div>{/if}
					</div>
				</div>
				<div class="header-viewport-wrapper pointer-events-none fixed inset-x-0 top-12">
					<NavigationMenu.Viewport onpointerenter={cancelHoverTimer} onpointerleave={leaveNavigation} class="frame-nav-viewport pointer-events-auto relative mx-auto mt-1.5 w-full max-w-6xl origin-top overflow-hidden data-[state=closed]:pointer-events-none" />
				</div>
			</NavigationMenu.Root>
		{:else}
			<div class={cn('transition-[background-color,backdrop-filter] duration-200', scrollY > 50 && 'bg-background/75 backdrop-blur')}><MobileHeader /></div>
		{/if}
	</header>
</div>

<style>
	.header-surface {
		transition: height 200ms cubic-bezier(0.4, 0, 0.2, 1), background-color 200ms cubic-bezier(0.4, 0, 0.2, 1), border-color 200ms cubic-bezier(0.4, 0, 0.2, 1), box-shadow 200ms cubic-bezier(0.4, 0, 0.2, 1), backdrop-filter 200ms cubic-bezier(0.4, 0, 0.2, 1);
	}

	.header-viewport-wrapper {
		perspective: 1200px;
	}

	.header-divider {
		height: 1px;
		background-image: linear-gradient(90deg, var(--foreground) 1px, transparent 1px);
		background-size: 4px 1px;
		opacity: 0.2;
	}

	:global(.frame-nav-viewport) {
		height: var(--bits-navigation-menu-viewport-height);
		transition: height 200ms cubic-bezier(0.4, 0, 0.2, 1);
	}

	@media (prefers-reduced-motion: no-preference) {
		:global(.frame-nav-viewport[data-state='open']) { animation: menu-scale-in 200ms ease-out; }
		:global(.frame-nav-viewport[data-state='closed']) { animation: menu-scale-out 160ms ease-out; }
		/* Bits tracks each panel's presence separately from the viewport animation. */
		:global(.frame-nav-content[data-state='closed']:not([data-motion^='to-'])) { animation: menu-content-hold 160ms linear; }
		:global(.frame-nav-content[data-motion='from-start']) { animation: menu-enter-left 200ms ease-out; }
		:global(.frame-nav-content[data-motion='from-end']) { animation: menu-enter-right 200ms ease-out; }
		:global(.frame-nav-content[data-motion='to-start']) { animation: menu-exit-left 160ms ease-out; }
		:global(.frame-nav-content[data-motion='to-end']) { animation: menu-exit-right 160ms ease-out; }
	}

	@media (prefers-reduced-motion: reduce) {
		.header-surface, :global(.frame-nav-viewport) { transition: none; }
	}

	@keyframes menu-scale-in { from { opacity: 0; transform: rotateX(-8deg) scale(0.96); } to { opacity: 1; transform: rotateX(0) scale(1); } }
	@keyframes menu-scale-out { from { opacity: 1; transform: rotateX(0) scale(1); } to { opacity: 0; transform: rotateX(-8deg) scale(0.96); } }
	@keyframes menu-content-hold { from, to { opacity: 1; } }
	@keyframes menu-enter-left { from { opacity: 0; transform: translateX(-1rem); } }
	@keyframes menu-enter-right { from { opacity: 0; transform: translateX(1rem); } }
	@keyframes menu-exit-left { to { opacity: 0; transform: translateX(-1rem); } }
	@keyframes menu-exit-right { to { opacity: 0; transform: translateX(1rem); } }
</style>
