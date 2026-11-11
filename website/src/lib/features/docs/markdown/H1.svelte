<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<h1
	{...restProps}
	class={cn(
		'font-heading text-3xl leading-tight font-medium tracking-tight text-balance text-foreground',
		className
	)}
>
	{@render children?.()}
</h1>
