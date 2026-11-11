<!--
Source: Hugeicons core-free-icons (existing adaptation with a filled center)
Original name: ReactIcon
https://www.npmjs.com/package/@hugeicons/core-free-icons
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { strokeWidth = 1.5, ...props }: IconProps = $props();
</script>

<IconRoot viewBox="0 0 24 24" stroke="currentColor" stroke-width={strokeWidth} {...props}>
	<path
		d="M12.0001 8C17.5229 8 22.0001 9.79086 22.0001 12C22.0001 14.2091 17.5229 16 12.0001 16C6.47721 16 2.00006 14.2091 2.00006 12C2.00006 9.79086 6.47721 8 12.0001 8Z"
	/>
	<path
		d="M15.5591 9.97529C18.3962 14.8173 19.1027 19.649 17.1371 20.7672C15.1715 21.8855 11.2781 18.8667 8.44104 14.0247C5.60395 9.18269 4.89746 4.35097 6.86306 3.23275C8.82865 2.11454 12.722 5.13327 15.5591 9.97529Z"
	/>
	<path
		d="M15.5591 14.0247C12.722 18.8667 8.82865 21.8855 6.86306 20.7672C4.89746 19.649 5.60395 14.8173 8.44104 9.97529C11.2781 5.13327 15.1715 2.11454 17.1371 3.23275C19.1027 4.35097 18.3962 9.18269 15.5591 14.0247Z"
	/>
	<circle cx="12" cy="12" r="0.75" fill="currentColor" stroke="none" />
</IconRoot>
