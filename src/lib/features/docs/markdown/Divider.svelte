<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import { Separator } from '$lib/components/ui/separator';
	import { cn } from '$lib/utils';

	type Props = ComponentProps<typeof Separator>;
	let { class: className, ...restProps }: Props = $props();
</script>

<Separator {...restProps} class={cn('my-12', className)} />
