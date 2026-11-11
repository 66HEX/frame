<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { ComponentProps } from 'svelte';
	import { siteConfig } from '$lib/site/site';
	import type { DocsNavigationGroup } from '$lib/content/docs';
	import SiteBrand from '$lib/components/layout/SiteBrand.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import ViewSwitcher from './view-switcher.svelte';
	import type { ContentSectionConfig } from '$lib/content/types';
	import type { SectionUiConfig } from '$lib/site/content-ui';
	import { GitHubIcon } from '$lib/icons';
	import NavMain from './nav-main.svelte';

	type Props = ComponentProps<typeof Sidebar.Root> & {
		groups: readonly DocsNavigationGroup[];
		section: ContentSectionConfig;
		ui: SectionUiConfig;
	};

	let { ref = $bindable(null), groups, section, ui, ...restProps }: Props = $props();
	const sidebar = Sidebar.useSidebar();

	const currentPathname = $derived(normalizePathname(page.url.pathname));
	const navigationGroups = $derived(
		groups.map((group) => ({
			id: group.id,
			title: group.title,
			items: group.items.map((item) => ({
				title: item.title,
				url: item.href,
				isActive: currentPathname === normalizePathname(item.href)
			}))
		}))
	);

	afterNavigate(() => {
		if (sidebar.isMobile) sidebar.setOpenMobile(false);
	});

	function normalizePathname(pathname: string) {
		const normalized = pathname.replace(/\/+$/, '');
		return normalized || '/';
	}
</script>

<Sidebar.Root bind:ref {...restProps}>
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton
					size="lg"
					class="hover:bg-transparent active:bg-transparent dark:hover:bg-transparent focus-visible:ring-ring/50"
					tooltipContent={siteConfig.name}
				>
					{#snippet child({ props })}
						<a {...props} href={resolve('/')} aria-label="Frame home">
							<SiteBrand />
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>

	<ViewSwitcher {section} />

	<Sidebar.Content
		style="mask-image: linear-gradient(to bottom, transparent, black 16px, black calc(100% - 16px), transparent); -webkit-mask-image: linear-gradient(to bottom, transparent, black 16px, black calc(100% - 16px), transparent);"
		class="overflow-hidden"
	>
		<NavMain groups={navigationGroups} label={ui.sidebar.navigationLabel} />
	</Sidebar.Content>
	{#if ui.sidebar.showRepositoryLink && siteConfig.links.github}
		<Sidebar.Footer>
			<Sidebar.Menu
				><Sidebar.MenuItem
					><Sidebar.MenuButton>
						{#snippet child({ props })}<a
								{...props}
								href={siteConfig.links.github}
								target="_blank"
								rel="noopener noreferrer"
								><GitHubIcon size={18} aria-hidden="true" /><span>{ui.sidebar.repositoryLabel}</span
								><span class="sr-only">{ui.pageActions.newTabHint}</span></a
							>{/snippet}
					</Sidebar.MenuButton></Sidebar.MenuItem
				></Sidebar.Menu
			>
		</Sidebar.Footer>
	{/if}
</Sidebar.Root>
