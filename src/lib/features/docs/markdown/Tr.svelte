<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import * as TablePrimitive from '$lib/components/ui/table';

	type Props = ComponentProps<typeof TablePrimitive.Row> & { children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<TablePrimitive.Row {...restProps} class={className}>
	{@render children?.()}
</TablePrimitive.Row>
