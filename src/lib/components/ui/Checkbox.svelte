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
			'peer appearance-none w-3.5 h-3.5 border border-gray-alpha-400 rounded bg-transparent checked:bg-ds-blue-600 checked:border-ds-blue-600 transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-alpha-400',
			className
		)}
		{...props}
	/>
	{#if checked && !indeterminate}
		<Check size={10} class="absolute text-foreground pointer-events-none opacity-100" />
	{/if}
</div>
