<script lang="ts">
	import type {
		ConversionConfig,
		MetadataStatus,
		OutputEstimate,
		SourceMetadata
	} from '$lib/types';
	import { estimateOutput } from '$lib/services/media';

	let {
		config,
		metadata,
		metadataStatus = 'idle'
	}: {
		config?: ConversionConfig | null;
		metadata?: SourceMetadata;
		metadataStatus?: MetadataStatus;
	} = $props();

	let estimate = $state<OutputEstimate | null>(null);
	let estimating = $state(false);
	let estimateError = $state<string | null>(null);

	$effect(() => {
		if (!config) {
			estimate = null;
			estimateError = null;
			return;
		}

		estimating = true;
		estimateError = null;
		const currentConfig = JSON.parse(JSON.stringify(config));
		const currentMetadata = metadata ? JSON.parse(JSON.stringify(metadata)) : undefined;
		let cancelled = false;

		estimateOutput(currentConfig, currentMetadata)
			.then((result) => {
				if (!cancelled) estimate = result;
			})
			.catch((error) => {
				if (!cancelled) {
					estimate = null;
					estimateError = error instanceof Error ? error.message : 'Unable to estimate';
				}
			})
			.finally(() => {
				if (!cancelled) estimating = false;
			});

		return () => {
			cancelled = true;
		};
	});

	function formatSize(sizeMb?: number) {
		if (!sizeMb) return '—';
		if (sizeMb >= 1024) return `${(sizeMb / 1024).toFixed(1)} GB`;
		return `${sizeMb.toFixed(1)} MB`;
	}
</script>

<div class="h-full">
	{#if !config}
		<div
			class="text-gray-alpha-600 flex h-full items-center justify-center rounded-lg border border-gray-alpha-100 bg-gray-alpha-100 px-4 text-center text-[10px] font-medium uppercase"
		>
			Select a file to view estimated output.
		</div>
	{:else}
		<div class="flex h-full flex-col rounded-lg border border-gray-alpha-100 bg-gray-alpha-100">
			<div class="flex h-10 items-center justify-between border-b border-gray-alpha-100 px-4">
				<div class="flex items-center">
					<span class="text-gray-alpha-600 text-[10px] font-medium tracking-widest uppercase"
						>Estimated Output</span
					>
				</div>
			</div>
			{#if metadataStatus === 'loading'}
				<div
					class="text-gray-alpha-600 flex flex-1 items-center justify-center text-[11px] tracking-wide uppercase"
				>
					Gathering metadata…
				</div>
			{:else}
				<div class="grid grid-cols-2 gap-2 p-4 text-[11px] uppercase">
					<div class="text-gray-alpha-600">Size</div>
					<div class="text-foreground">
						{#if estimating}…{:else}{formatSize(estimate?.sizeMb)}{/if}
					</div>
					<div class="text-gray-alpha-600">Video</div>
					<div class="text-foreground">
						{estimate ? `${estimate.videoKbps} kb/s` : '—'}
					</div>
					<div class="text-gray-alpha-600">Audio</div>
					<div class="text-foreground">
						{estimate ? (estimate.audioKbps > 0 ? `${estimate.audioKbps} kb/s` : '—') : '—'}
					</div>
					<div class="text-gray-alpha-600">Total</div>
					<div class="text-foreground">
						{estimate ? `${estimate.totalKbps} kb/s` : '—'}
					</div>
				</div>
				{#if estimateError}
					<div class="px-4 pb-3 text-[10px] text-ds-red-700 normal-case">
						{estimateError}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</div>
