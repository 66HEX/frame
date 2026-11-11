<script lang="ts">
	import { mergeProps } from "bits-ui";
	import { SidebarLeftClosedIcon, SidebarLeftIcon } from '$lib/icons';
	import { ActionTooltip } from '$lib/components/action-tooltip';
	import { IconTransition } from '$lib/components/icon-transition';
	import { contentUiDefaults } from '$lib/site/content-ui';
	import { keyboardShortcuts } from '$lib/site/keyboard-shortcuts';
	import { Button } from "$lib/components/ui/button/index.js";
	import { cn } from "$lib/utils.js";
	import { useSidebar } from "./context.svelte.js";
	import type { ComponentProps } from "svelte";

	let {
		ref = $bindable(null),
		class: className,
		onclick,
		...restProps
	}: ComponentProps<typeof Button> & {
		onclick?: (e: MouseEvent) => void;
	} = $props();

	const sidebar = useSidebar();
	const isOpen = $derived(sidebar.isMobile ? sidebar.openMobile : sidebar.open);
	const label = $derived(isOpen ? contentUiDefaults.shell.hideSidebar : contentUiDefaults.shell.showSidebar);

	function toggleSidebar(event: MouseEvent) {
		onclick?.(event);
		if (!event.defaultPrevented) sidebar.toggle();
	}
</script>

<ActionTooltip content={label} shortcut={keyboardShortcuts.sidebar.display} side="right">
	{#snippet trigger({ props })}
		<Button
			{...mergeProps(props, restProps, { onclick: toggleSidebar })}
			bind:ref
			data-sidebar="trigger"
			data-slot="sidebar-trigger"
			variant="ghost"
			size="icon-sm"
			class={cn(className)}
			type="button"
			aria-label={label}
			aria-expanded={isOpen}
			aria-haspopup={sidebar.isMobile ? 'dialog' : undefined}
			aria-keyshortcuts={keyboardShortcuts.sidebar.ariaKeyShortcuts}
		>
			<IconTransition
				active={isOpen}
				inactiveIcon={SidebarLeftClosedIcon}
				activeIcon={SidebarLeftIcon}
				class="size-[18px]"
				iconClass="size-[18px] cn-rtl-flip"
				size={18}
			/>
		</Button>
	{/snippet}
</ActionTooltip>
