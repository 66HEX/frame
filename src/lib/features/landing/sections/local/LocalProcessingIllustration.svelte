<script lang="ts">
	import { FileIcon } from '$lib/icons';
</script>

{#snippet fileStack(filename: string)}
	<div class="relative isolate min-w-0 flex-1 pt-4 sm:max-w-56 sm:pt-6">
		<div class="absolute inset-x-3 inset-y-0 rounded-xl bg-muted shadow-sm sm:inset-x-4"></div>
		<div
			class="absolute inset-x-1.5 top-2 bottom-0 rounded-xl bg-card shadow-sm sm:inset-x-2 sm:top-3"
		></div>
		<div class="relative rounded-xl bg-card p-2 shadow-sm sm:p-3">
			<div class="aspect-4/5 overflow-hidden rounded-md">
				<img
					src="/media/alpine-lake.jpg"
					alt=""
					width="1600"
					height="1067"
					loading="lazy"
					decoding="async"
					class="block size-full object-cover"
				/>
			</div>
			<div class="flex items-center gap-2 pt-3 pb-1 text-muted-foreground sm:pt-4">
				<FileIcon class="hidden size-4 shrink-0 sm:block" />
				<span class="font-mono text-xs leading-none whitespace-nowrap sm:text-sm">{filename}</span>
			</div>
		</div>
	</div>
{/snippet}

<figure
	class="grid min-h-64 min-w-0 place-items-center sm:min-h-80"
	aria-label="Source files converted into new formats by Frame, entirely on your computer"
	data-local-processing
>
	<div
		class="relative flex w-full max-w-2xl items-center justify-between gap-3 sm:gap-6"
		aria-hidden="true"
	>
		<div class="absolute inset-x-8 top-1/2 h-px bg-border"></div>
		{@render fileStack('clip.mov')}
		<img
			class="relative z-1 block size-16 shrink-0 object-contain sm:size-24 xl:size-28"
			src="/media/frame-app-icon.png"
			alt=""
			width="1024"
			height="1024"
			loading="lazy"
			decoding="async"
		/>
		{@render fileStack('clip.mp4')}
	</div>
</figure>
