<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		class: className,
		...props
	}: HTMLAttributes<HTMLHeadingElement> & { children: Snippet } = $props();
</script>

<h2
	class={cn(
		'text-title-4 font-medium text-balance md:text-title-5 lg:text-title-6',
		className
	)}
	{...props}
>
	{@render children()}
</h2>
