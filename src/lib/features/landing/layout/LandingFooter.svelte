<script lang="ts">
	import LandingBrand from './LandingBrand.svelte';
	import { githubUrl } from '../data/content';
	import Container from '../shared/Container.svelte';

	const linkClass = 'transition-colors duration-150 ease-in-out hover:text-foreground';
</script>

<footer class="border-t border-border">
	<Container
		class="grid grid-cols-2 gap-x-8 gap-y-12 py-16 sm:grid-cols-5 sm:py-20"
	>
		<div class="col-span-full sm:col-span-2">
			<a href="/" aria-label="Frame home"><LandingBrand /></a>
			<p class="mt-5 text-sm leading-6 text-muted-foreground">
				Native by design.<br />Open by default.
			</p>
		</div>
		<nav
			aria-label="Product links"
			class="flex flex-col items-start gap-3 text-sm text-muted-foreground"
		>
			<h2 class="mb-2 text-sm font-medium text-foreground">Product</h2>
			<a class={linkClass} href="/#features">Features</a><a class={linkClass} href="/#workflow"
				>Workflow</a
			><a class={linkClass} href="/#download">Download</a>
		</nav>
		<nav
			aria-label="Learn about Frame"
			class="flex flex-col items-start gap-3 text-sm text-muted-foreground"
		>
			<h2 class="mb-2 text-sm font-medium text-foreground">Resources</h2>
			<a class={linkClass} href="/docs">Documentation</a><a
				class={linkClass}
				href="/docs/getting-started">Getting started</a
			><a class={linkClass} href="/docs/installation">Installation</a>
		</nav>
		<nav
			aria-label="Project links"
			class="flex flex-col items-start gap-3 text-sm text-muted-foreground"
		>
			<h2 class="mb-2 text-sm font-medium text-foreground">Open source</h2>
			<a class={linkClass} href={githubUrl}>GitHub</a><a
				class={linkClass}
				href={`${githubUrl}/releases`}>Release notes</a
			><a class={linkClass} href={`${githubUrl}/issues`}>Report an issue</a><a
				class={linkClass}
				href={`${githubUrl}/blob/master/LICENSE`}>GPLv3 license</a
			>
		</nav>
	</Container>
	<Container class="flex flex-wrap justify-between gap-4 pb-8 text-xs text-muted-foreground"
		><span>© {new Date().getFullYear()} Frame</span><a
			href="https://madebyhex.com/"
			class="hover:text-foreground">Built by Marek Jóźwiak</a
		></Container
	>
</footer>
