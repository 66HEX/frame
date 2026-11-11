<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/ui-layout/18px_layout-left.svg
Source SVG SHA-256: 137cd2af8dcf7bab8f8d0904d4811353b5a117c80905e5f7a17d24fe3c67eacb
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<rect x="1.75" y="2.75" width="14.5" height="12.5" rx="2" ry="2" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</rect>
	<line x1="4.75" y1="5.75" x2="4.75" y2="12.25" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
</IconRoot>
