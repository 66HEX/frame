<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import { IconCheck } from '$lib/icons';
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

<div
	style="background-color: var(--background)"
	class={cn('button button-highlight relative flex items-center justify-center rounded', className)}
>
	<input
		type="checkbox"
		bind:this={ref}
		bind:checked
		{indeterminate}
		class={cn(
			'peer h-3.5 w-3.5 appearance-none rounded bg-transparent transition-colors checked:bg-blue-600 disabled:pointer-events-none disabled:opacity-50'
		)}
		{...props}
	/>
	{#if checked && !indeterminate}
		<IconCheck size={12} class="pointer-events-none absolute text-foreground opacity-100" />
	{/if}
</div>
