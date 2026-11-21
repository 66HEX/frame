<script lang="ts" generics="T = never">
	import { Tooltip as TooltipPrimitive } from "bits-ui";
	import { MediaQuery } from "svelte/reactivity";

	let {
		open = $bindable(false),
		disabled = false,
		...restProps
	}: TooltipPrimitive.RootProps<T> = $props();

	const isTouchDevice = new MediaQuery("(hover: none), (pointer: coarse)");

	$effect(() => {
		if (isTouchDevice.current && open) {
			open = false;
		}
	});
</script>

<TooltipPrimitive.Root
	bind:open
	disabled={disabled || isTouchDevice.current}
	{...restProps}
/>
