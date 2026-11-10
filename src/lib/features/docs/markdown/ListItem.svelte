<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & { class?: string; children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<li {...restProps} class={cn('ps-1 text-base leading-7 text-muted-foreground', className)}>
	{@render children?.()}
</li>
