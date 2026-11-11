<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { Accordion, Dialog } from 'bits-ui';
	import { Button } from '$lib/components/ui/button';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import { ChevronDownIcon, Menu2Icon, XmarkIcon } from '$lib/icons';
	import { githubUrl } from '../data/content';
	import HeaderMenuContent from './HeaderMenuContent.svelte';
	import LandingBrand from './LandingBrand.svelte';

	let open = $state(false);
	let expandedGroup = $state('');
	const menus = [
		{ value: 'product', label: 'Product' },
		{ value: 'resources', label: 'Resources' }
	] as const;
	const focusClass = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring';
	const iconButtonClass = `-mr-3 flex size-11 items-center justify-center rounded-lg text-foreground ${focusClass}`;

	function closeMenu() {
		open = false;
		expandedGroup = '';
	}

	afterNavigate(closeMenu);
</script>

<svelte:window
	onresize={() => {
		if (window.matchMedia('(min-width: 64rem)').matches) closeMenu();
	}}
/>

<Dialog.Root
	bind:open
	onOpenChange={(nextOpen) => {
		if (!nextOpen) expandedGroup = '';
	}}
>
	<div class="h-14 border-b border-border lg:hidden">
		<div class="mx-auto flex h-full w-full max-w-6xl items-center justify-between px-6">
			<a href="/" aria-label="Frame home" class={`rounded-sm ${focusClass}`}><LandingBrand /></a>
			<Dialog.Trigger aria-label="Open menu" class={iconButtonClass}>
				<Menu2Icon class="size-5" aria-hidden="true" />
			</Dialog.Trigger>
		</div>
	</div>
	<Dialog.Portal>
		<Dialog.Content
			data-frame-mobile-menu
			class="fixed inset-x-0 top-0 z-50 h-dvh overflow-y-auto overscroll-contain bg-background/75 text-foreground backdrop-blur"
		>
			<Dialog.Title class="sr-only">Frame navigation</Dialog.Title>
			<Dialog.Description class="sr-only">Explore Frame, find documentation, and download the app.</Dialog.Description>
			<div class="mx-auto min-h-full w-full max-w-6xl px-6 pb-8">
				<div class="flex h-14 items-center justify-between border-b border-border">
					<a href="/" aria-label="Frame home" class={`rounded-sm ${focusClass}`} onclick={closeMenu}><LandingBrand /></a>
					<Dialog.Close aria-label="Close menu" class={iconButtonClass}>
						<XmarkIcon class="size-5" aria-hidden="true" />
					</Dialog.Close>
				</div>
				{#if open}
					<nav aria-label="Mobile navigation" class="-mx-4">
						<Accordion.Root type="single" bind:value={expandedGroup}>
							{#each menus as menu (menu.value)}
								<Accordion.Item value={menu.value} class="relative after:pointer-events-none after:absolute after:inset-x-4 after:bottom-0 after:h-px after:bg-border">
									<Accordion.Header level={3}>
										<Accordion.Trigger class={`group flex min-h-14 w-full items-center justify-between rounded-md px-4 py-3 text-left text-lg leading-7 font-normal data-[state=open]:bg-foreground/5 ${focusClass}`}>
											{menu.label}
											<ChevronDownIcon class="size-4 shrink-0 text-muted-foreground group-data-[state=open]:rotate-180 motion-safe:transition-transform motion-safe:duration-200" aria-hidden="true" />
										</Accordion.Trigger>
									</Accordion.Header>
									<Accordion.Content data-frame-mobile-group class="overflow-hidden">
										<HeaderMenuContent menu={menu.value} mobile onNavigate={closeMenu} />
									</Accordion.Content>
								</Accordion.Item>
							{/each}
						</Accordion.Root>
						<div class="mx-4 border-b border-border">
							<a href="/docs" class={`flex min-h-15 items-center rounded-lg py-3 text-lg leading-7 ${focusClass}`} onclick={closeMenu}>Documentation</a>
						</div>
						<div class="mx-4 border-b border-border">
							<a href={githubUrl} class={`flex min-h-15 items-center rounded-lg py-3 text-lg leading-7 ${focusClass}`} onclick={closeMenu}>GitHub</a>
						</div>
					</nav>
					<div class="mt-6 flex items-center gap-3">
						<Button href="/#download" variant="outline" class="flex-1" onclick={closeMenu}>Download Frame</Button>
						<ThemeToggle showTooltip={false} />
					</div>
				{/if}
			</div>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	@media (prefers-reduced-motion: no-preference) {
		:global([data-frame-mobile-menu][data-state='open']) {
			animation: menu-expand 200ms cubic-bezier(0.4, 0, 0.2, 1);
		}

		:global([data-frame-mobile-menu][data-state='closed']) {
			animation: menu-collapse 200ms cubic-bezier(0.4, 0, 0.2, 1);
		}

		:global([data-frame-mobile-group][data-state='open']) {
			animation: group-expand 200ms ease-out;
		}

		:global([data-frame-mobile-group][data-state='closed']) {
			animation: group-collapse 200ms ease-out;
		}
	}

	@keyframes menu-expand {
		from {
			height: 3.5rem;
			background-color: transparent;
			backdrop-filter: blur(0);
			overflow-y: hidden;
		}
		to {
			height: 100dvh;
			background-color: color-mix(in oklab, var(--background) 75%, transparent);
			backdrop-filter: blur(8px);
			overflow-y: hidden;
		}
	}

	@keyframes menu-collapse {
		from {
			height: 100dvh;
			background-color: color-mix(in oklab, var(--background) 75%, transparent);
			backdrop-filter: blur(8px);
			overflow-y: hidden;
		}
		to {
			height: 3.5rem;
			background-color: transparent;
			backdrop-filter: blur(0);
			overflow-y: hidden;
		}
	}

	@keyframes group-expand {
		from { height: 0; }
		to { height: var(--bits-accordion-content-height); }
	}

	@keyframes group-collapse {
		from { height: var(--bits-accordion-content-height); }
		to { height: 0; }
	}
</style>
