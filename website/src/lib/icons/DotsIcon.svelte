<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/ui-layout/18px_dots.svg
Source SVG SHA-256: a58081f789f3032468de1abed3a2017b0506661d93d732dde57422241478ddb9
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<circle cx="9" cy="9" r=".5" fill="currentColor" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</circle>
	<circle cx="3.25" cy="9" r=".5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" fill="currentColor">
	</circle>
	<circle cx="14.75" cy="9" r=".5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" fill="currentColor">
	</circle>
</IconRoot>
