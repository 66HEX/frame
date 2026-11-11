<!--
Source: Nucleo UI 1.9.0 Fill Duo, via $lib/icons/CheckIcon.svelte
Original file: glyph-duo/ui-layout/18px_check.svg. The caller supplies the idle icon.
-->
<script lang="ts">
	import { IconTransition } from '$lib/components/icon-transition';
	import { CheckIcon, type IconComponent } from '$lib/icons';

	type Props = {
		copied: boolean;
		idleIcon: IconComponent;
	};

	let { copied, idleIcon }: Props = $props();
</script>

<IconTransition
	active={copied}
	inactiveIcon={idleIcon}
	activeIcon={CheckIcon}
	activeClass="text-emerald-500"
/>
