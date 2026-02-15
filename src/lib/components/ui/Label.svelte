<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLLabelAttributes } from 'svelte/elements';
	import { themeStore } from '$lib/stores/theme.svelte';

	const labelVariants = cva('text-gray-alpha-600 font-semibold', {
		variants: {
			variant: {
				default: 'text-[10px] block',
				section: 'text-[10px] pb-1 block w-full mb-3'
			}
		},
		defaultVariants: {
			variant: 'default'
		}
	});

	type Props = HTMLLabelAttributes &
		VariantProps<typeof labelVariants> & {
			ref?: HTMLLabelElement;
		};

	let { children, variant, class: className, ref = $bindable(), ...props }: Props = $props();
</script>

<label bind:this={ref} class={cn(labelVariants({ variant, className }))} {...props}>
		{@render children?.()}
		{#if variant === 'section'}
			<div
				class="mt-1 h-px [background-color:var(--divider-background)] shadow-2xs shadow-gray-alpha-100"
				style="--divider-background: color-mix(in srgb, var(--background), transparent {100 -
					themeStore.opacity}%)"
			></div>
		{/if}
</label>
