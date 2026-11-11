<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		class: className,
		...props
	}: HTMLAttributes<HTMLDivElement> & { children: Snippet } = $props();
</script>

<div class={cn('mx-auto w-full max-w-6xl px-6 md:px-10', className)} {...props}>
	{@render children()}
</div>
