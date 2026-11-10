<script lang="ts">
	import type { IconComponent } from '$lib/icons';
	import { cn } from '$lib/utils.js';

	type Props = {
		active: boolean;
		inactiveIcon: IconComponent;
		activeIcon: IconComponent;
		class?: string;
		inactiveClass?: string;
		activeClass?: string;
		iconClass?: string;
		size?: number | string;
	};

	let {
		active,
		inactiveIcon: InactiveIcon,
		activeIcon: ActiveIcon,
		class: className,
		inactiveClass,
		activeClass,
		iconClass,
		size = 16
	}: Props = $props();
</script>

<span
	class={cn('relative flex size-4 shrink-0 items-center justify-center leading-none', className)}
	aria-hidden="true"
>
	<span class={cn('icon-state', inactiveClass)} data-visible={!active}>
		<InactiveIcon class={iconClass} {size} />
	</span>
	<span class={cn('icon-state', activeClass)} data-visible={active}>
		<ActiveIcon class={iconClass} {size} />
	</span>
</span>

<style>
	.icon-state {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		transition-property: opacity, scale, filter;
		transition-duration: 150ms;
		will-change: transform;
		transition-timing-function: cubic-bezier(0.2, 0, 0, 1);
	}

	.icon-state[data-visible='true'] {
		scale: 1;
		opacity: 1;
		filter: blur(0);
	}

	.icon-state[data-visible='false'] {
		scale: 0.25;
		opacity: 0;
		filter: blur(4px);
	}

	@media (prefers-reduced-motion: reduce) {
		.icon-state {
			scale: 1;
			filter: none;
			transition-property: opacity;
		}
	}
</style>
