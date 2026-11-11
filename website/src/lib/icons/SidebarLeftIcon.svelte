<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/ui-layout/18px_sidebar-left.svg
Source SVG SHA-256: 16059e9621c4eaa438add82c8574a2e8a4d86014d3f44c6837f60d56a7a439b6
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<line x1="6.25" y1="2.75" x2="6.25" y2="15.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<rect x="1.75" y="2.75" width="14.5" height="12.5" rx="2" ry="2" transform="translate(18 18) rotate(180)" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</rect>
</IconRoot>
