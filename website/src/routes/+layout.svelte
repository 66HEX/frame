<script lang="ts">
	import '$lib/site/theme.css';
	import { page } from '$app/state';

	import { siteConfig } from '$lib/site/site';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { getContentSectionByPathname } from '$lib/content/sections';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { type Snippet } from 'svelte';

	const { children }: { children: Snippet } = $props();

	const currentPage = page;

	const isHomePath = (path?: string) => path === '/';

	const currentUrl = $derived(currentPage.url);
	const currentPath = $derived(currentUrl.pathname);
	const isHomeRoute = $derived(isHomePath(currentPath));
	const currentSection = $derived(getContentSectionByPathname(currentPath));
	const siteOrigin = new URL(siteConfig.url).origin;
	const canonicalUrl = $derived(new URL(currentPath, siteOrigin).href);

	const siteName = siteConfig.name;
	const authorName = siteConfig.author;
	const homeTitle = 'Frame — Every format. One workspace.';
	const homeDescription = siteConfig.description;
	const homeKeywords = siteConfig.keywords.join(', ');
	const sharedOgImage = $derived(new URL(siteConfig.ogImage, siteOrigin).href);
	const homeStructuredData = $derived.by(() =>
		JSON.stringify({
			'@context': 'https://schema.org',
			'@type': 'SoftwareApplication',
			name: siteName,
			alternateName: siteConfig.shortName,
			url: canonicalUrl,
			applicationCategory: 'MultimediaApplication',
			operatingSystem: 'macOS, Windows, Linux',
			description: homeDescription,
			image: sharedOgImage,
			offers: {
				'@type': 'Offer',
				price: '0',
				priceCurrency: 'USD'
			},
			provider: {
				'@type': 'Person',
				name: authorName
			}
		})
	);
</script>

<svelte:head>
	<meta
		name="theme-color"
		content={themeStore.isDark ? siteConfig.themeColor.dark : siteConfig.themeColor.light}
	/>
	<meta property="og:site_name" content={siteName} />
	<meta property="og:locale" content={siteConfig.locale} />
	<meta name="twitter:card" content="summary_large_image" />
	<link rel="icon" type="image/svg+xml" href={siteConfig.assets.favicon} />
	<link rel="apple-touch-icon" sizes="180x180" href={siteConfig.assets.appleTouchIcon} />
	<link rel="manifest" href="/site.webmanifest" />

	<meta name="mobile-web-app-capable" content="yes" />
	<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
	<meta name="apple-mobile-web-app-title" content={siteName} />
	{#if isHomeRoute}
		<title>{homeTitle}</title>
		<meta name="description" content={homeDescription} />
		<meta name="keywords" content={homeKeywords} />
		<meta name="author" content={authorName} />
		<link rel="canonical" href={canonicalUrl} />
		<meta property="og:title" content={homeTitle} />
		<meta property="og:description" content={homeDescription} />
		<meta property="og:type" content="website" />
		<meta property="og:url" content={canonicalUrl} />
		<meta property="og:image" content={sharedOgImage} />
		<meta property="og:image:alt" content={`${siteName} logomark`} />
		<meta property="og:image:type" content="image/png" />
		<meta name="twitter:title" content={homeTitle} />
		<meta name="twitter:description" content={homeDescription} />
		<meta name="twitter:image" content={sharedOgImage} />
		<svelte:element this={"script"} type="application/ld+json">
			{homeStructuredData}
		</svelte:element>
	{:else if !currentSection}
		<title>{siteName}</title>
		<meta name="description" content={homeDescription} />
		<link rel="canonical" href={canonicalUrl} />
	{/if}
</svelte:head>

<Tooltip.Provider delayDuration={350} skipDelayDuration={100}>
	{@render children()}
</Tooltip.Provider>
