<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import { Check } from 'lucide-svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';

	type Props = HTMLInputAttributes & {
		checked?: boolean;
		indeterminate?: boolean;
		ref?: HTMLInputElement;
	};

	let {
		checked = $bindable(false),
		indeterminate = false,
		class: className,
		ref = $bindable(),
		...props
	}: Props = $props();
</script>

<div class="relative flex items-center justify-center">
	<input
		type="checkbox"
		bind:this={ref}
		bind:checked
		{indeterminate}
		class={cn(
			'peer border-gray-alpha-400 h-3.5 w-3.5 cursor-pointer appearance-none rounded border bg-transparent transition-colors checked:border-ds-blue-600 checked:bg-ds-blue-600 disabled:pointer-events-none disabled:opacity-50',
			className
		)}
		{...props}
	/>
	{#if checked && !indeterminate}
		<Check size={10} class="pointer-events-none absolute text-foreground opacity-100" />
	{/if}
</div>
