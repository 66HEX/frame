<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	type Props = Record<string, unknown> & {
		id?: string;
		class?: string;
		children?: Snippet;
	};
	let { children, id, class: className, ...restProps }: Props = $props();
</script>

<h4
	{id}
	{...restProps}
	class={cn('mt-8 scroll-mt-24 text-base font-medium text-foreground', className)}
>
	{@render children?.()}
</h4>
