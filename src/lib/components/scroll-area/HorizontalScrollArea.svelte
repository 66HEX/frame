<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { cn } from '$lib/utils';
	import { observeHorizontalOverflow } from './horizontal-overflow';

	let {
		children,
		label,
		class: className
	}: {
		children: Snippet;
		label: string;
		class?: string;
	} = $props();
	let viewport = $state<HTMLElement | null>(null);

	$effect(() => {
		if (viewport) return observeHorizontalOverflow(viewport, label);
	});
</script>

<ScrollArea
	orientation="horizontal"
	bind:viewportRef={viewport}
	scrollbarXClasses="z-20"
	class={cn(
		'min-w-0 [&_[data-slot=scroll-area-viewport]]:focus-visible:ring-0 [&_[data-slot=scroll-area-viewport]]:focus-visible:outline-none',
		className
	)}
>
	{@render children()}
</ScrollArea>
