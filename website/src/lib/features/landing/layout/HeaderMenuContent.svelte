<script lang="ts">
	import {
		ArrowRightIcon,
		BookOpenIcon,
		CircleQuestionIcon,
		FileIcon,
		Grid2x2Icon,
		LayersIcon,
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
		{ title: 'Preview', description: 'Check your edits before export', href: '/docs/preview', icon: FileIcon },
		{ title: 'Batch conversion', description: 'Process files together', href: '/docs/batch-conversion', icon: LayersIcon },
		{ title: 'Presets', description: 'Save your output settings', href: '/docs/presets', icon: Grid2x2Icon }
	];

	const productSecondaryLinks = [
		{ title: 'Subtitles', href: '/docs/subtitles' },
		{ title: 'Metadata', href: '/docs/metadata' },
		{ title: 'Video filters', href: '/docs/video#video-filters' },
		{ title: 'Download Frame', href: '/#download' }
	];

	const resourceLeftLinks: MenuLink[] = [
		{ title: 'Getting started', description: 'Make your first conversion', href: '/docs/getting-started', icon: ArrowRightIcon },
		{ title: 'Installation', description: 'Set up Frame on your computer', href: '/docs/installation', icon: FileIcon }
	];

	const resourceMiddleLinks: MenuLink[] = [
		{ title: 'Documentation', description: 'Find guides for every workflow', href: '/docs', icon: BookOpenIcon },
		{ title: 'Troubleshooting', description: 'Get help with common issues', href: '/docs/troubleshooting', icon: CircleQuestionIcon }
	];

	const resourceMobileLinks: MenuLink[] = [...resourceLeftLinks, ...resourceMiddleLinks];

	const projectLinks = [
		{ title: 'GitHub', href: githubUrl },
		{ title: 'Releases', href: `${githubUrl}/releases` },
		{ title: 'Report an issue', href: `${githubUrl}/issues` }
	];

	const focusClass = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring';
	const mobileLinkClass = `flex min-h-10 items-center gap-3 rounded-md px-4 py-2 text-regular transition-colors hover:bg-muted/60 ${focusClass}`;
</script>

{#snippet mobileMenuLink(item: MenuLink)}
	<li>
		<a href={item.href} class={mobileLinkClass} onclick={() => onNavigate?.()}>
			<item.icon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
			<span>{item.title}</span>
		</a>
	</li>
{/snippet}

{#snippet mobileProjectLink(item: { title: string; href: string })}
	<li>
		<a href={item.href} class={mobileLinkClass} onclick={() => onNavigate?.()}>{item.title}</a>
	</li>
{/snippet}

{#snippet desktopRichLink(item: MenuLink)}
	<li>
		<a
			href={item.href}
			class={`linear-inline-link flex flex-col gap-1 rounded-lg p-4 text-mini transition-[background-color,color] duration-120 ease-out hover:bg-foreground/[0.04] ${focusClass}`}
			onclick={() => onNavigate?.()}
		>
			<span class="font-medium leading-none text-foreground">{item.title}</span>
			<span class="font-normal text-muted-foreground">{item.description}</span>
		</a>
	</li>
{/snippet}

{#snippet desktopCompactLink(item: { title: string; href: string })}
	<li>
		<a
			href={item.href}
			class={`linear-inline-link block rounded-lg px-4 py-2.5 text-mini font-medium leading-none text-foreground transition-[background-color,color] duration-120 ease-out hover:bg-foreground/[0.04] hover:text-foreground ${focusClass}`}
			onclick={() => onNavigate?.()}
		>
			{item.title}
		</a>
	</li>
{/snippet}

{#if mobile}
	<div class="pb-3">
		<ul aria-label={menu === 'product' ? 'Media' : 'Get started'}>
			{#each menu === 'product' ? mediaLinks : resourceMobileLinks as item (item.href)}
				{@render mobileMenuLink(item)}
			{/each}
		</ul>
		{#if menu === 'resources'}
			<p class="mt-4 px-4 pb-2 text-xs font-medium text-muted-foreground">Project</p>
			<ul aria-label="Project">
				{#each projectLinks as item (item.href)}
					{@render mobileProjectLink(item)}
				{/each}
			</ul>
		{/if}
	</div>
{:else}
	<div class="p-1">
		<div class="rounded-lg border border-border bg-card">
			<div
				class="linear-sub-items grid gap-x-6 p-4"
				style:grid-template-columns="244px 244px 192px"
			>
				{#if menu === 'product'}
					<ul aria-label="Media" class="linear-nav-column relative flex flex-col gap-0.5">
						{#each mediaLinks as item (item.href)}
							{@render desktopRichLink(item)}
						{/each}
					</ul>
					<ul aria-label="Tools" class="linear-nav-column relative flex flex-col gap-0.5">
						{#each toolLinks as item (item.href)}
							{@render desktopRichLink(item)}
						{/each}
					</ul>
					<ul aria-label="More features" class="linear-nav-column relative flex flex-col gap-0.5 pt-1">
						{#each productSecondaryLinks as item (item.href)}
							{@render desktopCompactLink(item)}
						{/each}
					</ul>
				{:else}
					<ul aria-label="Get started" class="linear-nav-column relative flex flex-col gap-0.5">
						{#each resourceLeftLinks as item (item.href)}
							{@render desktopRichLink(item)}
						{/each}
					</ul>
					<ul aria-label="Documentation" class="linear-nav-column relative flex flex-col gap-0.5">
						{#each resourceMiddleLinks as item (item.href)}
							{@render desktopRichLink(item)}
						{/each}
					</ul>
					<ul aria-label="Project" class="linear-nav-column relative flex flex-col gap-0.5 pt-1">
						{#each projectLinks as item (item.href)}
							{@render desktopCompactLink(item)}
						{/each}
					</ul>
				{/if}
			</div>
		</div>
		{#if menu === 'product'}
			<div class="linear-content-footer mt-1 flex overflow-hidden rounded-lg">
				<a
					href={releaseUrl}
					class={`flex min-w-0 flex-1 items-center justify-between gap-6 rounded-lg px-8 py-4 text-mini transition-[background-color] duration-120 ease-out hover:bg-foreground/[0.04] ${focusClass}`}
					onclick={() => onNavigate?.()}
				>
					<span class="flex min-w-0 items-center gap-2">
						<span class="font-medium text-foreground">New</span>
						<span class="truncate text-muted-foreground">What’s new in Frame</span>
					</span>
					<span class="flex shrink-0 items-center gap-1.5 text-muted-foreground">
						<span>Read release notes</span>
						<ArrowRightIcon class="size-3.5 text-muted-foreground/75" aria-hidden="true" />
					</span>
				</a>
			</div>
		{/if}
	</div>
{/if}

<style>
	.linear-sub-items {
		-webkit-mask-image: linear-gradient(90deg, transparent 0, #000 12px calc(100% - 12px), transparent 100%);
		mask-image: linear-gradient(90deg, transparent 0, #000 12px calc(100% - 12px), transparent 100%);
	}

	.linear-nav-column:not(:last-child)::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		right: -12px;
		width: 1px;
		background: var(--border);
	}
</style>
