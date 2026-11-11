<script lang="ts">
	import { Accordion as AccordionPrimitive } from "bits-ui";
	import { ChevronDownIcon } from '$lib/icons';
	import { cn, type WithoutChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		level = 3,
		children,
		...restProps
	}: WithoutChild<AccordionPrimitive.TriggerProps> & {
		level?: AccordionPrimitive.HeaderProps["level"];
	} = $props();
</script>

<AccordionPrimitive.Header {level} class="flex">
	<AccordionPrimitive.Trigger
		data-slot="accordion-trigger"
		bind:ref
		class={cn(
			"focus-visible:ring-ring/50 focus-visible:border-ring **:data-[slot=accordion-trigger-icon]:text-muted-foreground rounded-lg py-2.5 text-left text-sm font-medium hover:underline focus-visible:ring-3 **:data-[slot=accordion-trigger-icon]:ml-auto **:data-[slot=accordion-trigger-icon]:size-4 group/accordion-trigger relative flex flex-1 items-start justify-between border border-transparent transition-[color,box-shadow] motion-reduce:transition-none outline-none disabled:pointer-events-none disabled:opacity-50",
			className
		)}
		{...restProps}
	>
		{@render children?.()}
		<ChevronDownIcon
			data-slot="accordion-trigger-icon"
			aria-hidden="true"
			class="pointer-events-none shrink-0 transition-transform duration-200 ease-[ease-in-out] group-aria-expanded/accordion-trigger:rotate-180 motion-reduce:transition-none"
		/>
	</AccordionPrimitive.Trigger>
</AccordionPrimitive.Header>
