<script lang="ts">
	let { marker = false }: { marker?: boolean } = $props();
</script>

<div class="time-ruler relative mt-3 h-2 border-t border-border">
	{#if marker}
		<span class="absolute -top-1 -bottom-1 left-3/8 w-px bg-foreground"></span>
	{/if}
</div>

<style>
	/* One-pixel marks at sixteenth intervals retain the original drawing. */
	.time-ruler {
		background: repeating-linear-gradient(
			90deg,
			var(--border) 0 1px,
			transparent 1px calc(100% / 16)
		);
	}
</style>
