<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { loadWindowOpacity } from '$lib/services/settings';
	import { onMount } from 'svelte';

	type Props = HTMLInputAttributes & {
		ref?: HTMLInputElement;
	};

	let { class: className, value = $bindable(), ref = $bindable(), ...props }: Props = $props();

	onMount(() => {
		loadWindowOpacity().then((val) => {
			themeStore.opacity = val;
		});
	});
</script>

<div
	class="input-highlight relative flex h-8 w-full items-center rounded-sm border border-gray-alpha-200"
	style="background-color: color-mix(in srgb, var(--background), transparent {100 -
		themeStore.opacity}%)"
>
	<input
		bind:this={ref}
		bind:value
		class={cn(
			'flex w-full px-3 text-[10px] font-semibold transition-colors placeholder:text-gray-alpha-600 focus-visible:border-blue-700! focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
			'[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none',
			className
		)}
		{...props}
	/>
</div>
