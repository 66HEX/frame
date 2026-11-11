<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<blockquote
	{...restProps}
	class={cn(
		'mt-6 border-s-2 ps-6 text-base leading-7 text-muted-foreground italic [&>p]:mt-0',
		className
	)}
>
	{@render children?.()}
</blockquote>
