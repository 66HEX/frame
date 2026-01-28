<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	const buttonVariants = cva(
		'inline-flex items-center justify-center whitespace-nowrap rounded-sm text-[10px] font-medium uppercase tracking-wide transition-all disabled:pointer-events-none disabled:opacity-50',
		{
			variants: {
				variant: {
					default: 'bg-foreground text-background hover:bg-foreground/90',
					outline:
						'bg-transparent border border-gray-alpha-200 text-gray-alpha-600 hover:bg-gray-alpha-100 hover:text-foreground',
					secondary:
						'bg-gray-alpha-100 text-foreground border border-gray-alpha-100 hover:bg-gray-alpha-200',
					ghost: 'hover:bg-gray-alpha-100 text-gray-alpha-600 hover:text-foreground',
					'titlebar-ghost': 'text-gray-alpha-600 hover:text-foreground',
					selected: 'bg-ds-blue-900/20 text-ds-blue-600 border border-ds-blue-600',
					destructive: 'hover:text-ds-red-600',
					'titlebar-destructive': 'text-gray-alpha-600 hover:bg-ds-red-600 hover:text-foreground'
				},
				size: {
					default: 'h-7.5 px-3 py-1.5',
					sm: 'h-6 px-2',
					xs: 'h-6 px-2',
					icon: 'h-7.5 w-7.5',
					'icon-large': 'h-10 w-10',
					none: 'p-0'
				}
			},
			defaultVariants: {
				variant: 'default',
				size: 'default'
			}
		}
	);

	type Props = HTMLButtonAttributes &
		VariantProps<typeof buttonVariants> & {
			ref?: HTMLButtonElement;
		};

	let { children, variant, size, class: className, ref = $bindable(), ...props }: Props = $props();
</script>

<button bind:this={ref} class={cn(buttonVariants({ variant, size, className }))} {...props}>
	{@render children?.()}
</button>
