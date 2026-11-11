<script lang="ts">
	import { error } from '@sveltejs/kit';
	import TableOfContents from '$lib/features/docs/TableOfContents.svelte';
	import { resolveTocSelector } from '$lib/site/content-ui';
	import { siteConfig } from '$lib/site/site';
	import AppShell from '$lib/features/shell/AppShell.svelte';
	import DocsPageHeader from '$lib/features/docs/DocsPageHeader.svelte';
	import DocsPagination from '$lib/features/docs/DocsPagination.svelte';
	import { getNavigationGroups, getDocsDocument } from '$lib/content/docs';
	import {
		getContentSectionConfig,
		getContentSectionHref,
		getContentSectionManifest,
		getContentSectionUiConfig
	} from '$lib/content/sections';
	import { contentUiDefaults, type SectionUiConfig } from '$lib/site/content-ui';
	import type { LayoutData } from './$types';
	import type { Snippet } from 'svelte';

	const { data, children }: { data: LayoutData; children: Snippet } = $props();

	const sectionId = $derived(data.sectionId);
	const sectionUi = $derived<SectionUiConfig>(getContentSectionUiConfig(sectionId));
	const sectionConfig = $derived.by(() => {
		const config = getContentSectionConfig(sectionId);
		if (!config) error(404, 'View not found');
		return config;
	});
	const sectionManifest = $derived(getContentSectionManifest(sectionId));
	const navigationGroups = $derived(getNavigationGroups(sectionId));
	const isMarkdown = $derived(data.metadata?.sourceType === 'markdown');
	const sectionBasePath = $derived(`/${sectionId}`);

	const metadata = $derived(data.metadata);
	const docSlug = $derived(metadata?.slug);
	const currentDoc = $derived(sectionManifest.find((d) => d.slug === docSlug));

	const showPagination = $derived(
		sectionUi.pagination.enabled && currentDoc?.showPagination !== false
	);

	const previousLink = $derived(
		data.previousDoc
			? {
					title: data.previousDoc.name,
					href: getContentSectionHref(sectionId, data.previousDoc.slug)
				}
			: null
	);
	const nextLink = $derived(
		data.nextDoc
			? {
					title: data.nextDoc.name,
					href: getContentSectionHref(sectionId, data.nextDoc.slug)
				}
			: null
	);

	const siteOrigin = new URL(siteConfig.url).origin;
	const canonicalUrl = $derived(metadata ? new URL(metadata.href, siteOrigin).href : null);

	const docOgImage = $derived(
		metadata
			? new URL(`/${sectionId}/og/${metadata.slug || 'index'}`, siteOrigin).href
			: new URL(siteConfig.ogImage, siteOrigin).href
	);

	const docTitle = $derived(metadata?.title ?? currentDoc?.name ?? siteConfig.name);
	const docDescription = $derived(metadata?.description ?? siteConfig.description);

	const docStructuredData = $derived.by(() => {
		if (!canonicalUrl) return null;
		return JSON.stringify({
			'@context': 'https://schema.org',
			'@type': isMarkdown ? 'TechArticle' : 'WebPage',
			headline: docTitle,
			description: docDescription,
			url: canonicalUrl,
			author: {
				'@type': 'Person',
				name: siteConfig.author
			},
			publisher: {
				'@type': 'Organization',
				name: siteConfig.name
			},
			mainEntityOfPage: canonicalUrl
		});
	});

	const breadcrumbStructuredData = $derived.by(() => {
		if (!canonicalUrl) return null;
		return JSON.stringify({
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{
					'@type': 'ListItem',
					position: 1,
					name: contentUiDefaults.shell.homeLabel,
					item: siteOrigin
				},
				{
					'@type': 'ListItem',
					position: 2,
					name: sectionConfig.label,
					item: new URL(sectionBasePath, siteOrigin).href
				},
				{
					'@type': 'ListItem',
					position: 3,
					name: docTitle,
					item: canonicalUrl
				}
			]
		});
	});

	const doc = $derived(getDocsDocument(sectionId, metadata?.href ?? sectionBasePath));

	const tocSelector = $derived(resolveTocSelector(sectionUi.toc, docSlug));
	const scrollContainerId = $derived(`${sectionId}-content-container`);
</script>

<svelte:head>
	{#if metadata}
		<title>{docTitle} - {siteConfig.name}</title>
		<meta name="description" content={docDescription} />
		<link rel="canonical" href={canonicalUrl} />

		<meta property="og:type" content={isMarkdown ? 'article' : 'website'} />
		<meta property="og:title" content={docTitle} />
		<meta property="og:description" content={docDescription} />
		<meta property="og:url" content={canonicalUrl} />
		<meta property="og:image" content={docOgImage} />
		<meta property="og:image:alt" content={`${docTitle} — ${siteConfig.name}`} />
		<meta property="og:image:type" content="image/png" />
		<meta property="og:image:width" content="1200" />
		<meta property="og:image:height" content="630" />
		<meta name="twitter:card" content="summary_large_image" />
		<meta name="twitter:title" content={docTitle} />
		<meta name="twitter:description" content={docDescription} />
		<meta name="twitter:image" content={docOgImage} />
		{#if docStructuredData}
			<svelte:element this={"script"} type="application/ld+json">
				{docStructuredData}
			</svelte:element>
		{/if}
		{#if breadcrumbStructuredData}
			<svelte:element this={"script"} type="application/ld+json">
				{breadcrumbStructuredData}
			</svelte:element>
		{/if}
	{/if}
</svelte:head>

<AppShell
	{navigationGroups}
	{scrollContainerId}
	section={sectionConfig}
	ui={sectionUi}
	title={docTitle}
>
	{#snippet toc()}
		<div class="h-full min-h-0">
			<TableOfContents
				groupTitle={doc?.section ?? sectionConfig.label}
				selector={tocSelector}
				headings={data.tocHeadings}
				title={sectionUi.toc.title}
				emptyLabel={sectionUi.toc.emptyLabel}
				{scrollContainerId}
			/>
		</div>
	{/snippet}
	{#key metadata?.href}
		{#if isMarkdown && doc}<DocsPageHeader {doc} actions={sectionUi.pageActions} />{/if}
		{@render children()}
		{#if showPagination && sectionConfig.layout !== 'workspace'}<DocsPagination
				previous={previousLink}
				next={nextLink}
				config={sectionUi.pagination}
			/>{/if}
	{/key}
</AppShell>
