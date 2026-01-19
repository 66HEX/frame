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
			class="border bg-gray-alpha-100 border-gray-alpha-100 rounded-lg h-full flex items-center justify-center px-4 text-center text-[10px] font-medium text-gray-alpha-600 uppercase"
		>
			Select a file to view estimated output.
		</div>
	{:else}
		<div class="bg-gray-alpha-100 border border-gray-alpha-100 rounded-lg flex flex-col h-full">
			<div class="h-10 border-b border-gray-alpha-100 flex items-center justify-between px-4">
				<div class="flex items-center">
					<span class="text-[10px] font-medium uppercase tracking-widest text-gray-alpha-600"
						>Estimated Output</span
					>
				</div>
			</div>
			{#if metadataStatus === 'loading'}
				<div
					class="flex-1 flex items-center justify-center text-[11px] text-gray-alpha-600 uppercase tracking-wide"
				>
					Gathering metadata…
				</div>
			{:else}
				<div class="grid grid-cols-2 gap-2 text-[11px] p-4 uppercase">
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
					<div class="text-[10px] text-ds-red-700 normal-case px-4 pb-3">
						{estimateError}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</div>
