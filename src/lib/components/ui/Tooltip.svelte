<script lang="ts" module>
	let activeTooltipCount = 0;
	let skipDelayUntil = 0;
	let tooltipCounter = 0;
</script>

<script lang="ts">
	import { cva, type VariantProps } from 'class-variance-authority';
	import { scale } from 'svelte/transition';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils/cn';

	const tooltipContentVariants = cva(
		'pointer-events-none absolute z-50 rounded-sm bg-foreground px-2 py-1 text-[10px] font-semibold whitespace-nowrap normal-case! text-background shadow-lg',
		{
			variants: {
				side: {
					top: 'bottom-full left-1/2 mb-2 -translate-x-1/2',
					right: 'top-1/2 left-full ml-2 -translate-y-1/2',
					bottom: 'top-full left-1/2 mt-2 -translate-x-1/2',
					left: 'top-1/2 right-full mr-2 -translate-y-1/2'
				}
			},
			defaultVariants: {
				side: 'top'
			}
		}
	);

	const tooltipArrowVariants = cva('absolute h-0 w-0', {
		variants: {
			side: {
				top: 'top-full left-1/2 -translate-x-1/2 border-x-[5px] border-t-[5px] border-x-transparent border-t-foreground',
				right:
					'right-full top-1/2 -translate-y-1/2 border-y-[5px] border-r-[5px] border-y-transparent border-r-foreground',
				bottom:
					'bottom-full left-1/2 -translate-x-1/2 border-x-[5px] border-b-[5px] border-x-transparent border-b-foreground',
				left: 'left-full top-1/2 -translate-y-1/2 border-y-[5px] border-l-[5px] border-y-transparent border-l-foreground'
			}
		},
		defaultVariants: {
			side: 'top'
		}
	});

	type Side = NonNullable<VariantProps<typeof tooltipContentVariants>['side']>;

	type Props = {
		children?: Snippet;
		tooltip?: Snippet;
		content?: string;
		side?: Side;
		delay?: number;
		closeDelay?: number;
		switchGrace?: number;
		class?: string;
		tooltipClass?: string;
	};

	let {
		children,
		tooltip,
		content = '',
		side = 'top',
		delay = 600,
		closeDelay = 0,
		switchGrace = 200,
		class: className,
		tooltipClass
	}: Props = $props();

	let isOpen = $state(false);
	let isPointerInside = $state(false);
	let isFocusInside = $state(false);

	let openTimeout: ReturnType<typeof setTimeout> | undefined;
	let closeTimeout: ReturnType<typeof setTimeout> | undefined;

	const tooltipId = `tooltip-${++tooltipCounter}`;

	function clearOpenTimeout() {
		if (openTimeout) {
			clearTimeout(openTimeout);
			openTimeout = undefined;
		}
	}

	function clearCloseTimeout() {
		if (closeTimeout) {
			clearTimeout(closeTimeout);
			closeTimeout = undefined;
		}
	}

	function openNow() {
		if (isOpen) {
			return;
		}
		isOpen = true;
		activeTooltipCount += 1;
	}

	function closeNow() {
		if (!isOpen) {
			return;
		}
		isOpen = false;
		activeTooltipCount = Math.max(0, activeTooltipCount - 1);
		if (activeTooltipCount === 0) {
			skipDelayUntil = Date.now() + switchGrace;
		}
	}

	function shouldDelayOpen() {
		if (activeTooltipCount > 0) {
			return false;
		}
		return Date.now() >= skipDelayUntil;
	}

	function scheduleOpen() {
		clearCloseTimeout();
		clearOpenTimeout();

		const wait = shouldDelayOpen() ? delay : 0;
		if (wait <= 0) {
			openNow();
			return;
		}

		openTimeout = setTimeout(() => {
			openNow();
			openTimeout = undefined;
		}, wait);
	}

	function scheduleClose() {
		clearOpenTimeout();
		clearCloseTimeout();

		const wait = Math.max(0, closeDelay);
		if (wait === 0) {
			closeNow();
			return;
		}

		closeTimeout = setTimeout(() => {
			if (!isPointerInside && !isFocusInside) {
				closeNow();
			}
			closeTimeout = undefined;
		}, wait);
	}

	function onPointerEnter() {
		isPointerInside = true;
		scheduleOpen();
	}

	function onPointerLeave() {
		isPointerInside = false;
		if (!isFocusInside) {
			scheduleClose();
		}
	}

	function onFocusIn() {
		isFocusInside = true;
		scheduleOpen();
	}

	function onFocusOut(event: FocusEvent) {
		const currentTarget = event.currentTarget as HTMLElement | null;
		const next = event.relatedTarget as Node | null;
		if (currentTarget && next && currentTarget.contains(next)) {
			return;
		}
		isFocusInside = false;
		if (!isPointerInside) {
			scheduleClose();
		}
	}

	$effect(() => {
		return () => {
			clearOpenTimeout();
			clearCloseTimeout();
			closeNow();
		};
	});
</script>

<div
	class={cn('relative inline-flex', className)}
	onpointerenter={onPointerEnter}
	onpointerleave={onPointerLeave}
	onfocusin={onFocusIn}
	onfocusout={onFocusOut}
	aria-describedby={isOpen ? tooltipId : undefined}
>
	{@render children?.()}

	{#if isOpen && (content || tooltip)}
		<div
			id={tooltipId}
			role="tooltip"
			class={cn(tooltipContentVariants({ side }), tooltipClass)}
			transition:scale={{ start: 0.92, duration: 130 }}
		>
			{#if tooltip}
				{@render tooltip()}
			{:else}
				{content}
			{/if}
			<span class={tooltipArrowVariants({ side })}></span>
		</div>
	{/if}
</div>
