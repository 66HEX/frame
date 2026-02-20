<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	const buttonVariants = cva(
		'inline-flex items-center justify-center whitespace-nowrap rounded-sm text-[10px] font-semibold transition-all disabled:pointer-events-none disabled:transition-none',
		{
			variants: {
				variant: {
					default:
						'btn-primary disabled:text-foreground/50 disabled:opacity-50',
					secondary:
						'btn-secondary disabled:text-foreground/50 disabled:opacity-50',
					ghost:
						'hover:bg-gray-alpha-100 text-gray-alpha-600 hover:text-foreground disabled:bg-transparent disabled:opacity-50',
					'titlebar-ghost': 'text-gray-alpha-600 hover:text-foreground disabled:opacity-50',
					destructive:
						'btn-destructive disabled:opacity-50 disabled:text-red-700/60',
					'titlebar-destructive':
						'text-gray-alpha-600 hover:bg-red-700 hover:text-foreground disabled:opacity-50'
				},
				size: {
					default: 'h-7.5 px-3',
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
