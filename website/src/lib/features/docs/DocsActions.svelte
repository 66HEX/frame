<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import { CopyFeedbackIcon } from '$lib/components/copy-feedback';
	import type { SectionUiConfig } from '$lib/site/content-ui';
	import { siteConfig } from '$lib/site/site';
	import { Button } from '$lib/components/ui/button';
	import { ButtonGroup } from '$lib/components/ui/button-group';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { ChevronDownIcon, CopyIcon, FileIcon, GitHubIcon } from '$lib/icons';
	import { copyRawMarkdown } from './actions';

	type Props = {
		rawPath: string;
		config: SectionUiConfig['pageActions'];
	};

	type CopyState = 'idle' | 'copying' | 'success' | 'error';
	let { rawPath, config }: Props = $props();

	let copyState = $state<CopyState>('idle');
	let copyContentWidth = $state<number>();
	let copyWidthMotionReady = $state(false);
	let resetTimeout: ReturnType<typeof setTimeout> | undefined;

	const announcement = $derived(
		copyState === 'success'
			? config.copy.announcement
			: copyState === 'error'
				? config.copy.errorAnnouncement
				: ''
	);

	function copyLabel() {
		return config.copy[copyState];
	}

	const measureCopyContent: Attachment<HTMLSpanElement> = (node) => {
		let enableMotionFrame: number | undefined;
		let hasMeasured = false;

		const updateWidth = () => {
			// Keep the previous width while the shorter pending label is shown.
			if (hasMeasured && copyState === 'copying') return;

			const isInitialMeasurement = !hasMeasured;
			hasMeasured = true;
			copyContentWidth = Math.ceil(node.getBoundingClientRect().width);

			if (isInitialMeasurement) {
				enableMotionFrame = requestAnimationFrame(() => {
					copyWidthMotionReady = true;
				});
			}
		};
		const observer = new ResizeObserver(updateWidth);

		observer.observe(node);
		updateWidth();

		return () => {
			observer.disconnect();
			if (enableMotionFrame !== undefined) {
				cancelAnimationFrame(enableMotionFrame);
			}
		};
	};

	async function copyMarkdown() {
		if (copyState === 'copying') return;
		clearTimeout(resetTimeout);

		if (copyState !== 'success') {
			copyState = 'copying';
		}

		try {
			await copyRawMarkdown(rawPath);
			copyState = 'success';
		} catch {
			copyState = 'error';
		}

		resetTimeout = setTimeout(() => {
			copyState = 'idle';
		}, 2400);
	}

	onDestroy(() => clearTimeout(resetTimeout));
</script>

<ButtonGroup
	aria-label={config.label}
>
	{#if config.showCopyMarkdown}<Button
		size="sm"
		variant="default"
		class="active:not-aria-[haspopup]:scale-100"
		aria-busy={copyState === 'copying'}
		onclick={() => void copyMarkdown()}
	>
		<span
			class={[
				'-mx-1 inline-flex overflow-hidden',
				copyWidthMotionReady
					? 'transition-[width] duration-150 ease-out motion-reduce:transition-none'
					: 'transition-none'
			]}
			style:width={copyContentWidth === undefined ? undefined : `${copyContentWidth}px`}
		>
			<span
				class="inline-flex w-max shrink-0 items-center gap-1.5 px-1"
				{@attach measureCopyContent}
			>
				<CopyFeedbackIcon copied={copyState === 'success'} idleIcon={CopyIcon} />
				<span>{copyLabel()}</span>
			</span>
		</span>
	</Button>{/if}

	{#if config.showSourceLinks}
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						type="button"
						variant="default"
						size="icon-sm"
						aria-label={config.moreLabel}
					>
						<ChevronDownIcon aria-hidden="true" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>

			<DropdownMenu.Content align="start" class="w-auto">
				<DropdownMenu.Item>
					{#snippet child({ props })}
						<a {...props} href={rawPath} target="_blank" rel="noopener noreferrer">
							<FileIcon size={16} aria-hidden="true" />
							<span>View Markdown</span>
							<span class="sr-only">{config.newTabHint}</span>
						</a>
					{/snippet}
				</DropdownMenu.Item>
				<DropdownMenu.Item>
					{#snippet child({ props })}
						<a {...props} href={siteConfig.links.github} target="_blank" rel="noopener noreferrer">
							<GitHubIcon size={16} aria-hidden="true" />
							<span>View Frame on GitHub</span>
							<span class="sr-only">{config.newTabHint}</span>
						</a>
					{/snippet}
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	{/if}
</ButtonGroup>

<p class="sr-only" aria-live="polite" aria-atomic="true">{announcement}</p>
