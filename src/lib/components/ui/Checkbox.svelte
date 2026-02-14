<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import { IconCheck } from '$lib/icons';
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { loadWindowOpacity } from '$lib/services/settings';
	import { onMount } from 'svelte';

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

	onMount(() => {
		loadWindowOpacity().then((val) => {
			themeStore.opacity = val;
		});
	});
</script>

<div
	style="background-color: color-mix(in srgb, var(--background), transparent {100 -
		themeStore.opacity}%)"
	class="button button-highlight relative flex items-center justify-center rounded"
>
	<input
		type="checkbox"
		bind:this={ref}
		bind:checked
		{indeterminate}
		class={cn(
			'peer h-3.5 w-3.5 cursor-pointer appearance-none rounded bg-transparent transition-colors checked:border-blue-600 checked:bg-blue-600 disabled:pointer-events-none disabled:opacity-50',
			className
		)}
		{...props}
	/>
	{#if checked && !indeterminate}
		<IconCheck size={12} class="pointer-events-none absolute text-foreground opacity-100" />
	{/if}
</div>
