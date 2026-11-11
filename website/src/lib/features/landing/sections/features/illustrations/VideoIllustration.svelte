<script lang="ts">
	import TimeRuler from './TimeRuler.svelte';
</script>

<div class="w-full max-w-64">
	<div class="relative aspect-video overflow-hidden rounded-sm">
		<img
			src="/media/alpine-lake.jpg"
			alt=""
			width="1600"
			height="1067"
			loading="lazy"
			decoding="async"
			class="block size-full object-cover outline-1 -outline-offset-1 outline-border"
		/>
	</div>
	<TimeRuler marker />
</div>
