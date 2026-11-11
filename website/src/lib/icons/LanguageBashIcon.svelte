<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/design-development/18px_square-terminal.svg
Source SVG SHA-256: d9a3378295b6866b17adb4e04164516dac865485a4a1ed503552a9476773436f
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<rect x="2.75" y="2.75" width="12.5" height="12.5" rx="2" ry="2" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</rect>
	<line x1="9.75" y1="12.25" x2="12.25" y2="12.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<polyline points="5.75 12.25 8.25 9.75 5.75 7.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</polyline>
</IconRoot>
