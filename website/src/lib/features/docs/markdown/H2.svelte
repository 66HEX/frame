<script lang="ts">
	import type { Snippet } from 'svelte';
	import DocsHeadingAnchor from '../DocsHeadingAnchor.svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { id?: string; class?: string; children?: Snippet };
	let { children, id, class: className, ...restProps }: Props = $props();
</script>

<div class="group/heading mt-12 flex items-center gap-1">
	<h2
		{id}
		{...restProps}
		class={cn(
			'scroll-mt-24 font-heading text-2xl leading-tight font-medium tracking-tight text-balance text-foreground',
			className
		)}
	>
		{@render children?.()}
	</h2>
	{#if id}<DocsHeadingAnchor {id} />{/if}
</div>
