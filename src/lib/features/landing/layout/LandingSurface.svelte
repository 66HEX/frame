<script lang="ts">
	import type { Snippet } from 'svelte';
	let { children }: { children: Snippet } = $props();
</script>

<div
	class="landing-page min-h-screen bg-background text-foreground selection:bg-primary selection:text-primary-foreground [&_a:not([data-slot]):focus-visible]:rounded-sm [&_a:not([data-slot]):focus-visible]:outline-2 [&_a:not([data-slot]):focus-visible]:outline-offset-4 [&_a:not([data-slot]):focus-visible]:outline-ring"
>
	{@render children()}
</div>

<style>
	/* One scoped accessibility rule also covers pseudo-elements in child components. */
	@media (prefers-reduced-motion: reduce) {
		:global(.landing-page),
		:global(.landing-page *),
		:global(.landing-page::before),
		:global(.landing-page::after),
		:global(.landing-page *::before),
		:global(.landing-page *::after) {
			transition-duration: 0s !important;
			animation-duration: 0s !important;
		}
	}
</style>
