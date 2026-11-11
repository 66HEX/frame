<!--
Source: Spektral's preserved Cursor artwork; upstream asset attribution was not recorded
Original name: CursorIcon
https://github.com/kaltwrk/spektral/blob/e7a3551f/apps/web/src/lib/icons/CursorIcon.svelte
-->
<script lang="ts">
	import IconRoot from './IconRoot.svelte';
	import type { IconProps } from './types';

	let { ...props }: IconProps = $props();
</script>

<IconRoot viewBox="0 0 24 24" fill="currentColor" {...props}>
	<path
		d="M21.582 5.68 11.997.134a.994.994 0 0 0-.995 0L1.418 5.681A.84.84 0 0 0 1 6.407v11.186c0 .3.16.577.418.726l9.585 5.547a.994.994 0 0 0 .995 0l9.584-5.547a.84.84 0 0 0 .418-.726V6.407a.84.84 0 0 0-.418-.726Zm-.602 1.176L11.727 22.92c-.062.108-.227.064-.227-.061v-10.52a.59.59 0 0 0-.294-.51L2.119 6.57c-.108-.062-.064-.228.061-.228h18.504c.263 0 .427.286.296.514Z"
	/>
</IconRoot>
