<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<p
	{...restProps}
	class={cn('mt-4 max-w-prose text-base leading-7 text-muted-foreground', className)}
>
	{@render children?.()}
</p>
