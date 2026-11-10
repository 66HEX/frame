<script lang="ts">
	import { labels } from '$lib/site/labels';
	import * as Card from '$lib/components/ui/card';
	import HorizontalScrollArea from '$lib/components/scroll-area/HorizontalScrollArea.svelte';
	import { cn } from '$lib/utils';
	import CopyCodeButton from './CopyCodeButton.svelte';

	type Props = {
		code: string;
		htmlLight: string;
		htmlDark?: string | undefined;
		label?: string;
		lineNumbers?: boolean;
		compact?: boolean;
		framed?: boolean;
		copyable?: boolean;
		class?: string;
	};

	let {
		code,
		htmlLight,
		htmlDark,
		label = labels.code.example,
		lineNumbers = true,
		compact = false,
		framed = true,
		copyable = true,
		class: className
	}: Props = $props();
</script>

<Card.Root
	size={compact ? 'sm' : 'default'}
	class={cn(
		'code-block group/code-block relative gap-0 py-0',
		framed
			? 'transition-shadow has-[[data-scrollable]:focus-visible]:ring-ring/50 has-[[data-scrollable]:focus-visible]:ring-[3px] has-[[data-scrollable]:focus-visible]:outline-1'
			: 'rounded-lg shadow-none ring-0',
		className
	)}
>
	{#if copyable}
		<CopyCodeButton
			value={code}
			label={compact ? labels.code.copyInstall : labels.code.copyClipboard}
			class="absolute inset-e-2 top-2 z-10"
		/>
	{/if}

	<Card.Content class="px-0">
		<div
			class="code-content"
			class:compact
			class:with-line-numbers={lineNumbers}
			class:with-copy-action={copyable}
		>
			<div class="shiki-theme-light">
				<HorizontalScrollArea
					{label}
					class="code-scroll-area w-full min-w-0"
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html htmlLight}
				</HorizontalScrollArea>
			</div>
			<div class="shiki-theme-dark">
				<HorizontalScrollArea
					{label}
					class="code-scroll-area w-full min-w-0"
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html htmlDark ?? htmlLight}
				</HorizontalScrollArea>
			</div>
		</div>
	</Card.Content>
</Card.Root>

<style>
	.shiki-theme-dark {
		display: none;
	}

	:global(.dark) .shiki-theme-light {
		display: none;
	}

	:global(.dark) .shiki-theme-dark {
		display: block;
	}

	.code-content :global(.shiki) {
		margin: 0;
		overflow: visible;
		padding: 0.75rem;
		background-color: transparent !important;
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.5rem;
	}

	.code-content.compact :global(.shiki) {
		padding: 0.75rem;
	}

	.code-content :global(.shiki code) {
		display: block;
		width: max-content;
		min-width: 100%;
	}

	.code-content.with-copy-action :global(.shiki code) {
		padding-inline-end: 4rem;
	}

	.code-content.with-line-numbers :global(.shiki code) {
		counter-reset: line;
	}

	.code-content.with-line-numbers :global(.shiki .line) {
		counter-increment: line;
	}

	.code-content.with-line-numbers :global(.shiki .line::before) {
		display: inline-block;
		width: 1rem;
		margin-inline-end: 1rem;
		color: currentColor;
		text-align: end;
		content: counter(line);
		opacity: 0.25;
		user-select: none;
	}
</style>
