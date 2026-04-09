<script lang="ts">
	import type { SVGAttributes } from 'svelte/elements';
	let {
		size = 24,
		class: className,
		...props
	}: { size?: number | string; class?: string } & SVGAttributes<SVGElement> = $props();
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={size}
	height={size}
	viewBox="0 0 256 256"
	class={className}
	{...props}
>
	<rect width="256" height="256" fill="none" />
	<polyline
		points="80 176 128 224 176 176"
		fill="none"
		stroke="currentColor"
		stroke-linecap="round"
		stroke-linejoin="round"
		stroke-width="16"
	/>
	<polyline
		points="80 80 128 32 176 80"
		fill="none"
		stroke="currentColor"
		stroke-linecap="round"
		stroke-linejoin="round"
		stroke-width="16"
	/>
</svg>
