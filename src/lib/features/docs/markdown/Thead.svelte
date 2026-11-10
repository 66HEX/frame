<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import * as TablePrimitive from '$lib/components/ui/table';
	import { cn } from '$lib/utils';

	type Props = ComponentProps<typeof TablePrimitive.Header> & { children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<TablePrimitive.Header {...restProps} class={cn('[&_tr]:h-11 [&_tr]:border-0', className)}>
	{@render children?.()}
</TablePrimitive.Header>
