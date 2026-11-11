<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/arrows/18px_arrow-left.svg
Source SVG SHA-256: 787ddb34fdd83c370d77239c42bccc4fe06841ff610be3d001152920e0e827c6
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<line x1="2.75" y1="9" x2="15.25" y2="9" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<polyline points="7 13.25 2.75 9 7 4.75" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
</IconRoot>
