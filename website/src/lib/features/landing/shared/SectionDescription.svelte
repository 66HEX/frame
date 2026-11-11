<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		class: className,
		...props
	}: HTMLAttributes<HTMLParagraphElement> & { children: Snippet } = $props();
</script>

<p
	class={cn(
		'text-base leading-relaxed text-pretty text-muted-foreground md:leading-normal md:text-lg',
		className
	)}
	{...props}
>
	{@render children()}
</p>
