<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/design-development/18px_code.svg
Source SVG SHA-256: 4c2c195acdb141bd7c8fbe8a74a3a2852a3a7dcd75b6ed84c7d7361cf7f3aae0
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<polyline points="6.5 13.75 1.75 9 6.5 4.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
	<polyline points="11.5 13.75 16.25 9 11.5 4.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</polyline>
</IconRoot>
