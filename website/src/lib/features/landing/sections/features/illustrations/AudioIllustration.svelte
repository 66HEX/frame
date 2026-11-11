<script lang="ts">
	import TimeRuler from './TimeRuler.svelte';

	const waveform = [
		8, 12, 16, 10, 24, 34, 20, 42, 62, 46, 28, 54, 80, 96, 72, 44, 62, 84, 58, 32, 48, 68, 86, 62,
		40, 54, 34, 20, 30, 42, 28, 16, 22, 12, 8
	];
</script>

<div class="w-full max-w-64">
	<div class="relative flex aspect-video items-center justify-between">
		<div class="absolute inset-x-0 top-1/2 h-px bg-border"></div>
		{#each waveform as height, index (index)}
			<span class="relative w-0.5 rounded-full bg-muted-foreground" style:height={`${height}px`}
			></span>
		{/each}
	</div>
	<TimeRuler />
</div>
