<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	const buttonVariants = cva(
		'inline-flex items-center justify-center whitespace-nowrap rounded-sm text-[10px] font-medium uppercase tracking-wide transition-all disabled:pointer-events-none disabled:transition-none disabled:shadow-none',
		{
			variants: {
				variant: {
					default:
						'bg-foreground text-background hover:bg-foreground/90 disabled:bg-foreground disabled:text-background disabled:bg-foreground/50',
					outline:
						'bg-transparent border border-gray-alpha-200 text-gray-alpha-600 hover:bg-gray-alpha-100 hover:text-foreground disabled:bg-transparent disabled:text-gray-alpha-400 disabled:border-gray-alpha-100 disabled:hover:bg-transparent disabled:hover:text-gray-alpha-400',
					secondary:
						'bg-gray-alpha-100 text-foreground border border-gray-alpha-100 hover:bg-gray-alpha-200 disabled:bg-gray-alpha-100 disabled:text-foreground disabled:opacity-50',
					ghost:
						'hover:bg-gray-alpha-100 text-gray-alpha-600 hover:text-foreground border border-transparent disabled:bg-transparent disabled:opacity-50',
					'titlebar-ghost': 'text-gray-alpha-600 hover:text-foreground disabled:opacity-50',
					selected:
						'bg-blue-900/20 text-blue-600 border border-blue-600 disabled:bg-blue-900/10 disabled:border-blue-600/30 disabled:text-blue-600/50 disabled:hover:bg-blue-900/10',
					destructive: 'hover:text-red-600 disabled:opacity-50',
					'titlebar-destructive':
						'text-gray-alpha-600 hover:bg-red-600 hover:text-foreground disabled:opacity-50'
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
