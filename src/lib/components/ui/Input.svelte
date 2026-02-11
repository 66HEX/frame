<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { HTMLInputAttributes } from 'svelte/elements';

	type Props = HTMLInputAttributes & {
		ref?: HTMLInputElement;
	};

	let { class: className, value = $bindable(), ref = $bindable(), ...props }: Props = $props();
</script>

<input
	bind:this={ref}
	bind:value
	class={cn(
		'flex h-7.5 w-full rounded-sm border border-gray-alpha-200 bg-transparent px-3 py-1.5 text-[11px] tracking-wide transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-gray-alpha-400 focus-visible:border-blue-600! focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
		'[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none',
		className
	)}
	{...props}
/>
