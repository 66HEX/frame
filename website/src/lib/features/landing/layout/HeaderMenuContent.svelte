<script lang="ts">
	import { NavigationMenu } from 'bits-ui';
	import {
		ArrowRightIcon,
		BookOpenIcon,
		ChevronExpandYIcon,
		CircleQuestionIcon,
		FileIcon,
		Grid2x2Icon,
		LayersIcon,
		MagnifierIcon,
		TableOfContentsIcon,
		WorkflowIcon,
		type IconComponent
	} from '$lib/icons';
	import { githubUrl, releaseUrl } from '../data/content';

	let {
		menu,
		mobile = false,
		onNavigate
	}: {
		menu: 'product' | 'resources';
		mobile?: boolean;
		onNavigate?: () => void;
	} = $props();

	type MenuLink = {
		title: string;
		description: string;
		href: string;
		icon: IconComponent;
	};

	const mediaLinks: MenuLink[] = [
		{ title: 'Video', description: 'Codecs, quality, and resolution', href: '/docs/video', icon: FileIcon },
		{ title: 'Audio', description: 'Tracks, formats, and filters', href: '/docs/audio', icon: LayersIcon },
		{ title: 'Images', description: 'Resize, crop, and convert', href: '/docs/images', icon: Grid2x2Icon }
	];

	const toolLinks: MenuLink[] = [
		{ title: 'Preview', description: 'Check your edits before export', href: '/docs/preview', icon: MagnifierIcon },
		{ title: 'Batch conversion', description: 'Process files together', href: '/docs/batch-conversion', icon: WorkflowIcon },
		{ title: 'Presets', description: 'Save your output settings', href: '/docs/presets', icon: LayersIcon },
		{ title: 'Subtitles', description: 'Choose tracks or burn in text', href: '/docs/subtitles', icon: BookOpenIcon },
		{ title: 'Metadata', description: 'Keep the details you need', href: '/docs/metadata', icon: TableOfContentsIcon },
		{ title: 'Video filters', description: 'Adjust the picture', href: '/docs/video#video-filters', icon: ChevronExpandYIcon }
	];

	const resourceLinks: MenuLink[] = [
		{ title: 'Getting started', description: 'Make your first conversion', href: '/docs/getting-started', icon: ArrowRightIcon },
		{ title: 'Installation', description: 'Set up Frame on your computer', href: '/docs/installation', icon: FileIcon },
		{ title: 'Documentation', description: 'Find guides for every workflow', href: '/docs', icon: BookOpenIcon },
		{ title: 'Troubleshooting', description: 'Get help with common issues', href: '/docs/troubleshooting', icon: CircleQuestionIcon }
	];

	const projectLinks = [
		{ title: 'GitHub', href: githubUrl },
		{ title: 'Releases', href: `${githubUrl}/releases` },
		{ title: 'Report an issue', href: `${githubUrl}/issues` }
	];

	const focusClass = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring';
	const desktopLinkClass = `grid grid-cols-[auto_1fr] items-center gap-2.5 rounded-md p-2 transition-colors hover:bg-muted/60 ${focusClass}`;
	const mobileLinkClass = `flex min-h-10 items-center gap-3 rounded-md px-4 py-2 text-base leading-6 transition-colors hover:bg-muted/60 ${focusClass}`;
</script>

{#snippet linkContents(item: MenuLink)}
	{#if mobile}
		<item.icon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
		<span>{item.title}</span>
	{:else}
		<span class="flex size-9 items-center justify-center rounded-md bg-card text-muted-foreground shadow-sm">
			<item.icon class="size-4" aria-hidden="true" />
		</span>
		<span class="min-w-0">
			<span class="block text-sm leading-5 font-medium">{item.title}</span>
			<span class="mt-0.5 line-clamp-1 text-xs leading-4 text-muted-foreground">{item.description}</span>
		</span>
	{/if}
{/snippet}

{#snippet menuLink(item: MenuLink)}
	<li>
		{#if mobile}
			<a href={item.href} class={mobileLinkClass} onclick={() => onNavigate?.()}>
				{@render linkContents(item)}
			</a>
		{:else}
			<NavigationMenu.Link href={item.href} class={desktopLinkClass} onSelect={() => onNavigate?.()}>
				{@render linkContents(item)}
			</NavigationMenu.Link>
		{/if}
	</li>
{/snippet}

{#snippet projectLink(item: { title: string; href: string })}
	<li>
		{#if mobile}
			<a href={item.href} class={mobileLinkClass} onclick={() => onNavigate?.()}>{item.title}</a>
		{:else}
			<NavigationMenu.Link
				href={item.href}
				class={`block rounded-md px-2 py-2 text-sm leading-5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground ${focusClass}`}
				onSelect={() => onNavigate?.()}
			>{item.title}</NavigationMenu.Link>
		{/if}
	</li>
{/snippet}

{#if mobile}
	<div class="pb-3">
		<ul aria-label={menu === 'product' ? 'Media' : 'Get started'}>
			{#each menu === 'product' ? mediaLinks : resourceLinks as item (item.href)}
				{@render menuLink(item)}
			{/each}
		</ul>
		{#if menu === 'resources'}
			<p class="mt-4 px-4 pb-2 text-xs font-medium text-muted-foreground">Project</p>
			<ul aria-label="Project">
				{#each projectLinks as item (item.href)}
					{@render projectLink(item)}
				{/each}
			</ul>
		{/if}
	</div>
{:else}
	<div class="w-screen max-w-6xl pt-9 px-8 pb-8">
		<div class="grid grid-cols-4 gap-4">
			{#if menu === 'product'}
				<div class="-mr-2 border-r border-border pr-2">
					<p class="mb-2 px-2 text-xs leading-4 font-medium text-muted-foreground">Media</p>
					<ul aria-label="Media" class="space-y-2">
						{#each mediaLinks as item (item.href)}
							{@render menuLink(item)}
						{/each}
					</ul>
				</div>
				<div class="col-span-2">
					<p class="mb-2 px-2 text-xs leading-4 font-medium text-muted-foreground">Tools</p>
					<ul aria-label="Tools" class="grid grid-cols-2 gap-x-4 gap-y-2">
						{#each toolLinks as item (item.href)}
							{@render menuLink(item)}
						{/each}
					</ul>
				</div>
				<div>
					<p class="mb-4 px-2 text-xs leading-4 font-medium text-muted-foreground">Latest release</p>
					<NavigationMenu.Link
						href={releaseUrl}
						class={`group block overflow-hidden rounded-md bg-card shadow-sm ${focusClass}`}
						onSelect={() => onNavigate?.()}
					>
						<div class="h-26 overflow-hidden" aria-hidden="true">
							<img src="/media/landing/hero-light.webp" alt="" width="2400" height="1600" loading="lazy" decoding="async" class="size-full object-cover object-top dark:hidden" />
							<img src="/media/landing/hero-dark.webp" alt="" width="2400" height="1600" loading="lazy" decoding="async" class="hidden size-full object-cover object-top dark:block" />
						</div>
						<div class="p-3">
							<span class="flex items-center justify-between gap-2 text-sm leading-5 font-medium">What’s new in Frame <ArrowRightIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" /></span>
							<span class="mt-1 block text-xs leading-4 text-muted-foreground">Read the latest release notes.</span>
						</div>
					</NavigationMenu.Link>
				</div>
			{:else}
				<div class="col-span-2 border-r border-border pr-4">
					<p class="mb-2 px-2 text-xs leading-4 font-medium text-muted-foreground">Get started</p>
					<ul aria-label="Get started" class="grid grid-cols-2 gap-x-4 gap-y-2">
						{#each resourceLinks as item (item.href)}
							{@render menuLink(item)}
						{/each}
					</ul>
				</div>
				<div>
					<p class="mb-2 px-2 text-xs leading-4 font-medium text-muted-foreground">Project</p>
					<ul aria-label="Project">
						{#each projectLinks as item (item.href)}
							{@render projectLink(item)}
						{/each}
					</ul>
				</div>
			{/if}
		</div>
	</div>
{/if}
