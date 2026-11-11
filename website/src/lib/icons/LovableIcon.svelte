<!--
Source: SVGL (existing monochrome silhouette)
Original name: Lovable (lovable.svg)
https://github.com/pheralb/svgl/blob/main/static/library/lovable.svg
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot viewBox="0 0 121 122" fill="currentColor" {...props}>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M36.0687 0C55.9888 0 72.1373 16.1551 72.1373 36.0835V49.7975H84.141C104.061 49.7975 120.21 65.9526 120.21 85.8809C120.21 105.809 104.061 121.964 84.141 121.964H0V36.0835C0 16.1551 16.1485 0 36.0687 0Z"
	/>
</IconRoot>
