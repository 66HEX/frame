<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/arrows/18px_workflow.svg
Source SVG SHA-256: 73cf03add692556c1e962327ad1788dca36aafb71ed15c9539751ad9b71ef6f7
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<path d="m8.25,4.75c1.1046,0,2,.8954,2,2v4.5c0,1.1046.8954,2,2,2h3.75" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</path>
	<circle cx="3.75" cy="4.75" r="2" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</circle>
	<polyline points="13.5 10.5 16.25 13.25 13.5 16" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</polyline>
</IconRoot>
