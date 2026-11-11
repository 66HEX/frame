<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/arrows/18px_arrow-right.svg
Source SVG SHA-256: f6fae3759de7338ace9e2056a95ae2ded856778e2302275dbca4ff2631507528
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<line x1="15.25" y1="9" x2="2.75" y2="9" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<polyline points="11 4.75 15.25 9 11 13.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
</IconRoot>
