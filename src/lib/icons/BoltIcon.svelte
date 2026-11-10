<!--
Source: preserved Spektral Bolt.new logo; upstream asset attribution was not recorded
Original name: BoltIcon
https://github.com/kaltwrk/spektral/blob/e7a3551f/apps/web/src/lib/icons/BoltIcon.svelte
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot viewBox="0 0 16 16" fill="currentColor" {...props}>
	<path d="M7.398 9.091h-3.58L10.364 2 8.602 6.909h3.58L5.636 14l1.762-4.909Z" />
</IconRoot>
