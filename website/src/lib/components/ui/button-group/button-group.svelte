<script lang="ts" module>
	import { tv, type VariantProps } from "tailwind-variants";

	export const buttonGroupVariants = tv({
		base: "has-[>[data-slot=button-group]]:gap-2 has-[select[aria-hidden=true]:last-child]:[&>[data-slot=select-trigger]:last-of-type]:rounded-r-lg group/button-group flex w-fit items-stretch *:focus-visible:relative *:focus-visible:z-10 [&>[data-slot=select-trigger]:not([class*='w-'])]:w-fit [&>input]:flex-1",
		variants: {
			orientation: {
				horizontal:
					"[&>[data-slot]:not(:has(~[data-slot]))]:rounded-r-[var(--button-group-radius,var(--radius-lg))]! [&>*:not(:first-child)]:rounded-l-none [&>*:not(:last-child)]:rounded-r-none",
				vertical:
					"[&>[data-slot]:not(:has(~[data-slot]))]:rounded-b-[var(--button-group-radius,var(--radius-lg))]! flex-col [&>*:not(:first-child)]:rounded-t-none [&>*:not(:last-child)]:rounded-b-none",
			},
		},
		defaultVariants: {
			orientation: "horizontal",
		},
	});

	export type ButtonGroupOrientation = VariantProps<typeof buttonGroupVariants>["orientation"];
</script>

<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		children,
		orientation = "horizontal",
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		orientation?: ButtonGroupOrientation;
	} = $props();
</script>

<div
	bind:this={ref}
	role="group"
	data-slot="button-group"
	data-orientation={orientation}
	class={cn(buttonGroupVariants({ orientation }), className)}
	{...restProps}
>
	{@render children?.()}
</div>
