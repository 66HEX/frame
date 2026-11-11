<script lang="ts">
	import { labels } from '$lib/site/labels';
	import type { ComponentProps, Snippet } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import * as TablePrimitive from '$lib/components/ui/table';
	import HorizontalScrollArea from '$lib/components/scroll-area/HorizontalScrollArea.svelte';
	import { cn } from '$lib/utils';

	type Props = ComponentProps<typeof TablePrimitive.Root> & { children?: Snippet };
	let { children, class: className, ...restProps }: Props = $props();
</script>

<Card.Root size="sm" class="my-6 gap-0 bg-muted px-1 pt-0 pb-1 transition-shadow has-[[data-scrollable]:focus-visible]:ring-ring/50 has-[[data-scrollable]:focus-visible]:ring-[3px] has-[[data-scrollable]:focus-visible]:outline-1">
	<div class="relative min-w-0">
		<div
			class="pointer-events-none absolute inset-x-0 top-11 bottom-0 rounded-lg bg-card shadow-sm"
			aria-hidden="true"
		></div>

		<HorizontalScrollArea
			label={labels.code.scrollableTable}
			class="[&_[data-slot=table-container]]:overflow-visible"
		>
			<TablePrimitive.Root {...restProps} class={cn('relative z-10 min-w-xl', className)}>
				{@render children?.()}
			</TablePrimitive.Root>
		</HorizontalScrollArea>
	</div>
</Card.Root>
