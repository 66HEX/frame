<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';

	let {
		children,
		divided = false,
		class: className,
		...props
	}: HTMLAttributes<HTMLElement> & {
		children: Snippet;
		divided?: boolean;
	} = $props();
</script>

<section
	class={cn(
		'relative isolate mx-auto w-full max-w-6xl scroll-mt-18 px-6 py-20 md:px-10 md:py-28 lg:py-32',
		"even:after:pointer-events-none even:after:absolute even:after:inset-y-0 even:after:left-1/2 even:after:-z-10 even:after:w-screen even:after:-translate-x-1/2 even:after:bg-muted/50 even:after:content-[''] dark:even:after:bg-muted/20",
		divided &&
			"before:pointer-events-none before:absolute before:top-0 before:left-1/2 before:h-px before:w-screen before:-translate-x-1/2 before:bg-border before:content-['']",
		className
	)}
	{...props}
>
	{@render children()}
</section>
