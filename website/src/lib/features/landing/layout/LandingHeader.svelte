<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import { Menu2Icon, XmarkIcon } from '$lib/icons';
	import LandingBrand from './LandingBrand.svelte';
	import { navigation, githubUrl } from '../data/content';
	import Container from '../shared/Container.svelte';

	let mobileOpen = $state(false);
	let menuButton = $state<HTMLButtonElement | null>(null);
	function closeMenu() {
		mobileOpen = false;
		menuButton?.focus();
	}
	afterNavigate(() => {
		mobileOpen = false;
	});
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key === 'Escape' && mobileOpen) closeMenu();
	}}
/>

<header class="sticky top-0 z-40 border-b border-border bg-background/90 backdrop-blur-xl">
	<Container class="flex h-18 items-center justify-between gap-4 md:gap-6">
		<a href="/" aria-label="Frame home"><LandingBrand /></a>
		<div class="flex items-center gap-6 lg:gap-10">
			<nav aria-label="Primary navigation" class="hidden items-center gap-6 md:flex">
				{#each navigation as item (item.href)}
					<a
						href={item.href}
						class="flex min-h-11 items-center text-sm text-muted-foreground transition-colors hover:text-foreground"
						>{item.label}</a
					>
				{/each}
				<a
					href={githubUrl}
					class="flex min-h-11 items-center text-sm text-muted-foreground transition-colors hover:text-foreground"
					>GitHub</a
				>
			</nav>
			<div class="flex items-center gap-2">
				<Button href="/#download">Download</Button>
				<ThemeToggle />
				<Button
					bind:ref={menuButton}
					variant="ghost"
					size="icon"
					class="md:hidden"
					aria-label={mobileOpen ? 'Close menu' : 'Open menu'}
					aria-expanded={mobileOpen}
					aria-controls="mobile-navigation"
					onclick={() => (mobileOpen = !mobileOpen)}
				>
					{#if mobileOpen}<XmarkIcon />{:else}<Menu2Icon />{/if}
				</Button>
			</div>
		</div>
	</Container>
	<nav
		id="mobile-navigation"
		hidden={!mobileOpen}
		aria-label="Mobile navigation"
		class="mx-auto w-full max-w-7xl px-6 pb-6 md:hidden md:px-10"
	>
		{#each [...navigation, { label: 'GitHub', href: githubUrl }] as item (item.href)}
			<a
				href={item.href}
				onclick={() => (mobileOpen = false)}
				class="flex min-h-12 items-center text-sm text-muted-foreground hover:text-foreground"
				>{item.label}</a
			>
		{/each}
	</nav>
</header>
