<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import * as TablePrimitive from '$lib/components/ui/table';

	type Props = ComponentProps<typeof TablePrimitive.Body> & { children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<TablePrimitive.Body {...restProps} class={className}>
	{@render children?.()}
</TablePrimitive.Body>
