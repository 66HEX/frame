<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import * as TablePrimitive from '$lib/components/ui/table';
	import { cn } from '$lib/utils';

	type Props = ComponentProps<typeof TablePrimitive.Head> & { children?: Snippet };
	let { children, scope = 'col', class: className, ...restProps }: Props = $props();
</script>

<TablePrimitive.Head {scope} {...restProps} class={cn('py-0', className)}>
	{@render children?.()}
</TablePrimitive.Head>
