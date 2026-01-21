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
		'w-full flex items-center justify-between text-[11px] font-medium uppercase tracking-wide py-1.5 pr-3 border-l-2 transition-all duration-200 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-alpha-400 disabled:pointer-events-none disabled:opacity-50',
		selected
			? 'border-l-ds-blue-600 bg-gray-alpha-100 pl-3 text-foreground'
			: 'border-l-transparent bg-transparent pl-2 text-gray-alpha-600 hover:text-foreground',
		className
	)}
	{...props}
>
	{@render children?.()}
</button>
