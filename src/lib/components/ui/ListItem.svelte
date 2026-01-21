<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	type Props = HTMLButtonAttributes & {
		selected?: boolean;
		ref?: HTMLButtonElement;
	};

	let {
		children,
		selected = false,
		class: className,
		ref = $bindable(),
		...props
	}: Props = $props();
</script>

<button
	bind:this={ref}
	class={cn(
		'focus-visible:ring-gray-alpha-400 flex w-full items-center justify-between border-l-2 py-1.5 pr-3 text-[11px] font-medium tracking-wide uppercase transition-all duration-200 focus-visible:ring-1 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
		selected
			? 'border-l-ds-blue-600 bg-gray-alpha-100 pl-3 text-foreground'
			: 'text-gray-alpha-600 border-l-transparent bg-transparent pl-2 hover:text-foreground',
		className
	)}
	{...props}
>
	{@render children?.()}
</button>
