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
		'text-3xl font-medium tracking-tight text-balance md:text-4xl',
		className
	)}
	{...props}
>
	{@render children()}
</h2>
