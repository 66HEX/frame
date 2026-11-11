<script lang="ts">
	import LandingLayout from '$lib/features/landing/layout/LandingLayout.svelte';
	import Container from '$lib/features/landing/shared/Container.svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Button } from '$lib/components/ui/button';
	import { ArrowLeftIcon, ArrowRotateClockwiseIcon, BookOpenIcon } from '$lib/icons';
	import { siteConfig } from '$lib/site/site';

	const status = $derived(page.status);
	const canRetry = $derived(status >= 500 || status === 408 || status === 429);
	const message = $derived.by(() => {
		switch (status) {
			case 404:
				return {
					title: 'Page not found.',
					description:
						'This address doesn’t lead to a page. Head back home or find what you need in the docs.'
				};
			case 403:
				return {
					title: 'This page is restricted.',
					description:
						'You don’t have access to this page. You can still explore the public docs or return home.'
				};
			case 408:
				return {
					title: 'The request timed out.',
					description: 'This page took too long to respond. Reload it to try again.'
				};
			case 410:
				return {
					title: 'This page is no longer available.',
					description: 'Head back home or browse the docs to find the current information.'
				};
			case 429:
				return {
					title: 'Too many requests.',
					description: 'Please wait a moment before reloading this page.'
				};
			default:
				return status >= 500
					? {
							title: 'Unable to load this page.',
							description:
								'Something went wrong while loading this page. Try reloading it, or return to the homepage.'
						}
					: {
							title: 'This page is unavailable.',
							description: 'Check the address, or continue from the homepage or docs.'
						};
		}
	});
</script>

<svelte:head>
	<title>{status} · {message.title} — {siteConfig.name}</title>
	<meta name="robots" content="noindex, nofollow" />
	<meta name="description" content={message.description} />
</svelte:head>

<LandingLayout>
	<section aria-labelledby="error-title">
		<Container class="flex min-h-128 flex-col items-start justify-center py-20 md:py-28 lg:py-32">
			<p class="mb-6 text-sm font-medium text-muted-foreground tabular-nums" aria-hidden="true">
				{status}
			</p>
			<h1
				id="error-title"
				class="max-w-4xl text-4xl leading-none font-medium tracking-tight text-balance md:text-5xl lg:text-6xl"
			>
				<span class="sr-only">Error {status}: </span>{message.title}
			</h1>
			<p class="mt-6 max-w-xl text-base leading-relaxed text-pretty text-muted-foreground">
				{message.description}
			</p>
			<div class="mt-8 flex flex-wrap items-center gap-3" data-sveltekit-reload>
				{#if canRetry}
					<Button href={page.url.href} size="lg">
						<ArrowRotateClockwiseIcon aria-hidden="true" />
						Reload page
					</Button>
					<Button href={resolve('/')} variant="outline" size="lg">
						<ArrowLeftIcon aria-hidden="true" />
						Back to home
					</Button>
				{:else}
					<Button href={resolve('/')} size="lg">
						<ArrowLeftIcon aria-hidden="true" />
						Back to home
					</Button>
					<Button
						href={resolve('/[section]/[...slug]', { section: 'docs', slug: '' })}
						variant="outline"
						size="lg"
					>
						<BookOpenIcon aria-hidden="true" />
						Browse docs
					</Button>
				{/if}
			</div>
		</Container>
	</section>
</LandingLayout>
