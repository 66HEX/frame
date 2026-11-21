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
		'text-regular text-pretty text-muted-foreground md:text-large',
		className
	)}
	{...props}
>
	{@render children()}
</p>
