<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<code
	{...restProps}
	class={cn(
		'wrap-break-words inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-md bg-card px-2 py-0.5 text-xs font-medium shadow-xs transition-colors hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:bg-input/30',
		className
	)}
>
	{@render children?.()}
</code>
