<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { cn } from '$lib/utils/cn';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	const buttonVariants = cva(
		'inline-flex items-center justify-center whitespace-nowrap rounded-sm text-[10px] transition-all disabled:pointer-events-none disabled:transition-none',
		{
			variants: {
				variant: {
					default:
						'bg-frame-gray-400 text-foreground button-highlight text-shadow hover:bg-frame-gray-400/90 disabled:text-foreground/50 disabled:bg-frame-gray-400/50 disabled:hover:bg-frame-gray-400/50',
					secondary:
						'bg-frame-gray-100 button-highlight text-shadow text-foreground hover:bg-frame-gray-200 disabled:bg-frame-gray-100 disabled:text-foreground/50 disabled:opacity-50',
					ghost:
						'hover:bg-frame-gray-100 text-frame-gray-600 hover:text-foreground disabled:bg-transparent disabled:opacity-50',
					'titlebar-ghost': 'text-frame-gray-600 hover:text-foreground disabled:opacity-50',
					destructive:
						'bg-frame-gray-100 button-highlight text-shadow text-frame-red disabled:opacity-50 hover:bg-frame-gray-200 disabled:bg-frame-gray-100 disabled:text-frame-red/50 ',
					'titlebar-destructive':
						'text-frame-gray-600 hover:bg-frame-red hover:text-foreground disabled:opacity-50'
				},
				size: {
					default: 'h-7.5 px-2.5',
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
