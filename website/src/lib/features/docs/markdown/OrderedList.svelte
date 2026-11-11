<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<ol
	{...restProps}
	class={cn(
		'mt-4 max-w-prose list-decimal space-y-2 ps-5 marker:font-mono marker:text-xs marker:text-muted-foreground',
		className
	)}
>
	{@render children?.()}
</ol>
