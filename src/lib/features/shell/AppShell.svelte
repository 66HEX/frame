<script lang="ts">
	import { afterNavigate, beforeNavigate } from '$app/navigation';
	import { tick } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { Snippet } from 'svelte';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { ThemeToggle } from '$lib/components/theme-toggle';
	import PageContainer from '$lib/components/layout/PageContainer.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import type { DocsNavigationGroup } from '$lib/content/docs';
	import DocsSearch from '$lib/features/docs/DocsSearch.svelte';
	import type { ContentSectionConfig } from '$lib/content/types';
	import { contentUiDefaults, type SectionUiConfig } from '$lib/site/content-ui';
	import Separator from '$lib/components/ui/separator/separator.svelte';

	type Props = {
		children: Snippet;
		toc: Snippet;
		section: ContentSectionConfig;
		ui: SectionUiConfig;
		title: string;
		scrollContainerId: string;
		navigationGroups: readonly DocsNavigationGroup[];
	};

	let { children, toc, scrollContainerId, navigationGroups, section, ui, title }: Props = $props();

	const currentDoc = $derived(
		navigationGroups
			.flatMap((group) => group.items)
			.find((doc) => doc.pathname === page.url.pathname)
	);
	const sectionLabel = $derived(currentDoc?.section ?? section.label);
	const documentLabel = $derived(title);
	let docsViewport = $state<HTMLElement | null>(null);

	const scrollPositions = new SvelteMap<string, number>();
	beforeNavigate(({ from }) => {
		if (from && docsViewport) scrollPositions.set(from.url.pathname, docsViewport.scrollTop);
	});

	afterNavigate(({ from, to, type }) => {
		void tick().then(async () => {
			if (to?.url.hash) {
				// Resolve the destination after fonts finish changing the article's line wrapping.
				await document.fonts.ready;
				if (page.url.href !== to.url.href) return;
				document
					.getElementById(decodeURIComponent(to.url.hash.slice(1)))
					?.scrollIntoView({ block: 'start' });
			} else if (from && to && from.url.pathname !== to.url.pathname) {
				docsViewport?.scrollTo({
					top: type === 'popstate' ? (scrollPositions.get(to.url.pathname) ?? 0) : 0,
					left: 0,
					behavior: 'instant'
				});
				window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
			}
		});
	});

	$effect(() => {
		if (docsViewport) docsViewport.id = scrollContainerId;
	});
</script>

<Button
	href="#docs-content"
	variant="secondary"
	class="fixed inset-s-2 top-2 z-50 -translate-y-20 focus:translate-y-0"
>
	{contentUiDefaults.shell.skipLink}
</Button>

<Sidebar.Provider
	data-app-shell
	class="fixed inset-[4px] min-h-0 w-auto overflow-clip rounded-xl border border-foreground/20 bg-background dark:border-border"
>
	<AppSidebar {section} {ui} groups={navigationGroups} class="md:absolute md:h-full" />

	<Sidebar.Inset class="min-h-0 min-w-0 overflow-hidden overscroll-none">
		<header class="relative flex shrink-0 items-center gap-2 border-b border-border py-3">
			<div class="flex min-w-0 flex-1 items-center gap-4 px-4">
				<Sidebar.Trigger />
				{#if ui.search.enabled}{#key section.id}<DocsSearch
							sectionId={section.id}
							config={ui.search}
						/>{/key}{/if}
				<Separator orientation="vertical" class="my-auto data-[orientation=vertical]:h-4" />
				<Breadcrumb.Root class="min-w-0">
					<Breadcrumb.List class="flex-nowrap">
						<Breadcrumb.Item class="hidden md:block">
							<Breadcrumb.Link
								href={resolve('/[section]/[...slug]', { section: section.id, slug: '' })}
								>{section.label}</Breadcrumb.Link
							>
						</Breadcrumb.Item>
						<Breadcrumb.Separator class="hidden md:block" />
						{#if sectionLabel !== section.label}
							<Breadcrumb.Item class="hidden md:block">
								<span>{sectionLabel}</span>
							</Breadcrumb.Item>
							<Breadcrumb.Separator class="hidden md:block" />
						{/if}
						<Breadcrumb.Item class="min-w-0">
							<Breadcrumb.Page class="truncate">{documentLabel}</Breadcrumb.Page>
						</Breadcrumb.Item>
					</Breadcrumb.List>
				</Breadcrumb.Root>

				<div class="ms-auto flex shrink-0 items-center gap-2">
					{#if contentUiDefaults.theme.showToggle}<ThemeToggle />{/if}
				</div>
			</div>
		</header>

		<div class="[container-type:size] min-h-0 flex-1 overflow-clip">
			{#if section.layout === 'workspace'}
				<main
					id="docs-content"
					tabindex="-1"
					class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden outline-none"
				>
					{@render children()}
				</main>
			{:else}
				<!-- The article and its sticky TOC share one viewport, with the scrollbar at its outer edge. -->
				<ScrollArea
					data-docs-scroll-area
					class="h-full min-w-0"
					viewportClass="scroll-smooth motion-reduce:scroll-auto"
					bind:viewportRef={docsViewport}
				>
					<div class="grid min-h-full min-w-0 md:grid-cols-[minmax(0,1fr)_18rem]">
						<aside
							id="docs-toc-sidebar"
							aria-label={ui.toc.title}
							class="sticky top-0 z-10 min-w-0 self-start border-b border-border bg-background/90 text-sidebar-foreground backdrop-blur-md md:z-auto md:col-start-2 md:row-start-1 md:flex md:h-[100cqh] md:flex-col md:border-b-0 md:bg-background md:p-6 md:backdrop-blur-none"
						>
							{@render toc()}
						</aside>
						<PageContainer
							class="min-w-0 pt-10 pb-16 sm:pt-14 md:col-start-1 md:row-start-1 lg:pt-16 lg:pb-24"
						>
							<main
								id="docs-content"
								tabindex="-1"
								class="mx-auto max-w-2xl text-pretty outline-none lg:max-w-3xl"
							>
								{@render children()}
							</main>
						</PageContainer>
					</div>
				</ScrollArea>
			{/if}
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
