<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { HTMLInputAttributes } from 'svelte/elements';

	type Props = HTMLInputAttributes & {
		ref?: HTMLInputElement;
	};

	let { class: className, value = $bindable(), ref = $bindable(), ...props }: Props = $props();
</script>

<div class="input-highlight relative flex h-7.5 w-full items-center rounded-sm bg-background">
	<input
		bind:this={ref}
		bind:value
		class={cn(
			'flex h-full w-full px-2.5 text-[10px] transition-colors placeholder:text-frame-gray-600 disabled:pointer-events-none disabled:opacity-50',
			'[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none',
			className
		)}
		{...props}
	/>
</div>
