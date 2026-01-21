<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { HTMLInputAttributes } from 'svelte/elements';

	type Props = HTMLInputAttributes & {
		value?: number;
		min?: number;
		max?: number;
		step?: number;
		ref?: HTMLInputElement;
	};

	let {
		class: className,
		value = $bindable(0),
		min = 0,
		max = 100,
		step = 1,
		ref = $bindable(),
		...props
	}: Props = $props();

	// Calculate percentage for the fill bar
	let percentage = $derived(((value - min) / (max - min)) * 100);
</script>

<div class={cn('relative h-1.5 w-full rounded-full bg-gray-alpha-100', className)}>
	<!-- Fill Track -->
	<div
		class="absolute top-0 left-0 h-full rounded-full bg-foreground"
		style="width: {Math.max(0, Math.min(100, percentage))}%"
	></div>

	<!-- Invisible Input Overlay -->
	<input
		type="range"
		bind:this={ref}
		bind:value
		{min}
		{max}
		{step}
		class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
		{...props}
	/>
</div>
