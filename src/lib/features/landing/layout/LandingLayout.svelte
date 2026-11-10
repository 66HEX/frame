<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import LandingSurface from './LandingSurface.svelte';
	import LandingHeader from './LandingHeader.svelte';
	import LandingFooter from './LandingFooter.svelte';

	let { children }: { children: Snippet } = $props();
</script>

<LandingSurface>
	<Button
		href="#main-content"
		variant="secondary"
		class="fixed top-3 left-3 z-50 -translate-y-24 focus:translate-y-0">Skip to main content</Button
	>
	<LandingHeader />
	<main id="main-content" tabindex="-1">{@render children()}</main>
	<LandingFooter />
</LandingSurface>
