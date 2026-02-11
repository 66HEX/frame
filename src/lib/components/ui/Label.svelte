<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLLabelAttributes } from 'svelte/elements';

	const labelVariants = cva('tracking-widest text-gray-alpha-600 font-medium', {
		variants: {
			variant: {
				default: 'text-[9px] block',
				section: 'text-[10px] border-b border-gray-alpha-100 pb-1 block w-full mb-3'
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
</label>
