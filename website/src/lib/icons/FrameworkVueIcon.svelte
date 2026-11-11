<!--
Source: Spektral's preserved Vue artwork; upstream asset attribution was not recorded
Original name: AppFrameworkVueIcon
https://github.com/kaltwrk/spektral/blob/01498e0d/apps/web/src/lib/components/app-icons/AppFrameworkVueIcon.svelte
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { strokeWidth = 1.5, ...props }: IconProps = $props();
</script>

<IconRoot
	viewBox="0 0 24 24"
	stroke="currentColor"
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	{...props}
>
	<path d="M3 4.5h4.2L12 13l4.8-8.5H21l-9 15.7-9-15.7Z" />
	<path d="M7.5 4.5h3.1L12 7l1.4-2.5h3.1" />
</IconRoot>
