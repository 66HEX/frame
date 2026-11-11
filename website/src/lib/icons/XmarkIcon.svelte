<!--
Source: Nucleo UI 1.9.0, Outline (outline), 18px
Original file: outline/ui-layout/18px_xmark.svg
Source SVG SHA-256: d83361581afa2cd1df9605280a57bf0f8762890ffeb76cc81477d50e6944b292
https://nucleoapp.com/
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot {...props}>
	<line x1="14" y1="4" x2="4" y2="14" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" data-color="color-2">
	</line>
	<line x1="4" y1="4" x2="14" y2="14" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5">
	</line>
</IconRoot>
