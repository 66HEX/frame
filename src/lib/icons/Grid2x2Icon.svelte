<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/ui-layout/18px_grid-2x2.svg
Source SVG SHA-256: 6708d9478a536a79397017b0126bb2ea9809bda22d1e899e85484cf5e6d166ec
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<line x1="15.75" y1="9" x2="2.25" y2="9" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<line x1="9" y1="15.75" x2="9" y2="2.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</line>
</IconRoot>
