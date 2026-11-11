<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/arrows/18px_chevron-expand-y.svg
Source SVG SHA-256: c551943ae72c1d98125d71d5dd05bda0a884a8b224c63fdab7b0875f12da3c2c
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<polyline points="12.5 6.25 9 2.75 5.5 6.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
	<polyline points="12.5 11.75 9 15.25 5.5 11.75" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</polyline>
</IconRoot>
