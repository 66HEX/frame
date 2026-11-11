<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/design-development/18px_terminal.svg
Source SVG SHA-256: 58393118ae631bc440fa185f885d67895473a47c0b9d2cd9bee65a3e358f69da
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<polyline points="2.75 14.25 8 9 2.75 3.75" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
	<line x1="9.5" y1="14.25" x2="15.25" y2="14.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
</IconRoot>
