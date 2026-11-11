<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		class: className,
		...props
	}: HTMLAnchorAttributes & { children: Snippet } = $props();
</script>

<a
	class={cn(
		'inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground',
		className
	)}
	{...props}
>
	{@render children()}
</a>
