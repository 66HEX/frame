<script lang="ts">
	import { onDestroy, type Snippet } from 'svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';

	type Props = {
		content: string;
		shortcut?: string;
		trigger: Snippet<[{ props: Record<string, unknown> }]>;
		side?: 'top' | 'right' | 'bottom' | 'left';
		disableCloseOnTriggerClick?: boolean;
	};

	let {
		content,
		shortcut,
		trigger,
		side = 'top',
		disableCloseOnTriggerClick = false
	}: Props = $props();
	let open = $state(false);
	let protectedFocusTarget: HTMLElement | undefined;
	let cancelFocusProtection: (() => void) | undefined;

	function scrollAffectsTarget(eventTarget: EventTarget | null, target: HTMLElement) {
		const ownerWindow = target.ownerDocument.defaultView;

		if (!ownerWindow) return false;
		if (eventTarget === ownerWindow || eventTarget === target.ownerDocument) return true;

		return eventTarget instanceof ownerWindow.Node && eventTarget.contains(target);
	}

	function shouldKeepOpen() {
		const target = protectedFocusTarget;

		return (
			target !== undefined &&
			target.ownerDocument.activeElement === target &&
			target.matches(':focus-visible')
		);
	}

	function setOpen(nextOpen: boolean) {
		if (!nextOpen && shouldKeepOpen()) return;
		open = nextOpen;
	}

	function protectDuringFocusScroll(event: FocusEvent) {
		const target = event.currentTarget;

		if (!(target instanceof HTMLElement) || !target.matches(':focus-visible')) return;

		cancelFocusProtection?.();

		const ownerWindow = target.ownerDocument.defaultView;
		if (!ownerWindow) return;
		const focusTarget: HTMLElement = target;
		const focusWindow: Window = ownerWindow;

		let sawScroll = false;
		let settleTimer: number | undefined;
		let detectionTimer: number | undefined;
		let expiryTimer: number | undefined;

		function cleanup() {
			focusWindow.removeEventListener('scroll', handleScroll, true);
			focusWindow.removeEventListener('scrollend', handleScrollEnd, true);
			if (settleTimer !== undefined) focusWindow.clearTimeout(settleTimer);
			if (detectionTimer !== undefined) focusWindow.clearTimeout(detectionTimer);
			if (expiryTimer !== undefined) focusWindow.clearTimeout(expiryTimer);
			if (protectedFocusTarget === focusTarget) protectedFocusTarget = undefined;
			if (cancelFocusProtection === cleanup) cancelFocusProtection = undefined;
		}

		function handleScroll(scrollEvent: Event) {
			if (!scrollAffectsTarget(scrollEvent.target, focusTarget)) return;

			sawScroll = true;
			if (detectionTimer !== undefined) focusWindow.clearTimeout(detectionTimer);
			if (settleTimer !== undefined) focusWindow.clearTimeout(settleTimer);
			settleTimer = focusWindow.setTimeout(cleanup, 100);
		}

		function handleScrollEnd(scrollEvent: Event) {
			if (!sawScroll || !scrollAffectsTarget(scrollEvent.target, focusTarget)) return;
			cleanup();
		}

		protectedFocusTarget = focusTarget;
		focusWindow.addEventListener('scroll', handleScroll, true);
		focusWindow.addEventListener('scrollend', handleScrollEnd, true);

		// Drop the temporary protection quickly when focus did not cause scrolling,
		// and cap it in case a browser does not dispatch `scrollend`.
		detectionTimer = focusWindow.setTimeout(() => {
			if (!sawScroll) cleanup();
		}, 100);
		expiryTimer = focusWindow.setTimeout(cleanup, 1500);

		cancelFocusProtection = cleanup;
	}

	function withFocusRecovery(props: Record<string, unknown>) {
		const onfocus = props['onfocus'] as ((event: FocusEvent) => void) | undefined;
		const onblur = props['onblur'] as ((event: FocusEvent) => void) | undefined;

		return {
			...props,
			onfocus: (event: FocusEvent) => {
				protectDuringFocusScroll(event);
				onfocus?.(event);
			},
			onblur: (event: FocusEvent) => {
				cancelFocusProtection?.();
				onblur?.(event);
			}
		};
	}

	onDestroy(() => cancelFocusProtection?.());
</script>

<Tooltip.Root bind:open={() => open, setOpen} {disableCloseOnTriggerClick}>
	<Tooltip.Trigger>
		{#snippet child({ props })}
			{@render trigger({ props: withFocusRecovery(props) })}
		{/snippet}
	</Tooltip.Trigger>
	<Tooltip.Content {side}>
		<span>{content}</span>
		{#if shortcut}
			<kbd
				data-slot="kbd"
				class="bg-background/15 px-1.5 py-0.5 font-mono text-[10px] leading-none text-background/80"
			>
				{shortcut}
			</kbd>
		{/if}
	</Tooltip.Content>
</Tooltip.Root>
