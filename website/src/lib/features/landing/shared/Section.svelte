<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		divided = false,
		class: className,
		...props
	}: HTMLAttributes<HTMLElement> & {
		children: Snippet;
		divided?: boolean;
	} = $props();
</script>

<section
	class={cn(
		'mx-auto w-full max-w-7xl scroll-mt-18 px-6 py-20 md:px-10 md:py-28 lg:py-32',
		divided && 'border-t border-border',
		className
	)}
	{...props}
>
	{@render children()}
</section>
