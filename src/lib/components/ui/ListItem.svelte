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
		'flex h-7.5 w-full items-center justify-between rounded-sm border-l-2 pr-3 text-[10px] transition-all disabled:pointer-events-none disabled:opacity-50',
		selected
			? 'border-l-frame-gray-600 bg-frame-gray-100 pl-3 text-foreground'
			: 'border-l-transparent bg-transparent pl-2 text-frame-gray-600 hover:text-foreground',
		className
	)}
	{...props}
>
	{@render children?.()}
</button>
