<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<ul
	{...restProps}
	class={cn('mt-4 max-w-prose list-disc space-y-2 ps-5 marker:text-muted-foreground/70', className)}
>
	{@render children?.()}
</ul>
