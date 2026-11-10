<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import * as TablePrimitive from '$lib/components/ui/table';

	type Props = ComponentProps<typeof TablePrimitive.Cell> & { children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<TablePrimitive.Cell {...restProps} class={className}>
	{@render children?.()}
</TablePrimitive.Cell>
