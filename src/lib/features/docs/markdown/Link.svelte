<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<a
	{...restProps}
	class={cn(
		'rounded-sm font-medium text-foreground underline decoration-border decoration-1 underline-offset-4 transition-[color,background-color,border-color,box-shadow,scale,background-size] outline-none hover:decoration-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 motion-reduce:transition-none',
		className
	)}
>
	{@render children?.()}
</a>
