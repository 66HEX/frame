<script lang="ts">
	import type { MetadataStatus, SourceMetadata } from '$lib/types';

	let {
		metadata,
		status = 'idle',
		error
	}: {
		metadata?: SourceMetadata;
		status?: MetadataStatus;
		error?: string;
	} = $props();

	function display(value?: string) {
		return value && value.trim().length > 0 ? value : '—';
	}

	function formatDuration(raw?: string): string {
		if (!raw) return '—';
		const seconds = parseFloat(raw);
		if (isNaN(seconds)) return raw;

		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = Math.floor(seconds % 60);

		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}`;
	}

	function formatBitrate(raw?: string): string {
		if (!raw) return '—';
		const bps = parseFloat(raw);
		if (isNaN(bps)) return raw;

		if (bps >= 1_000_000) {
			return `${(bps / 1_000_000).toFixed(1)} Mb/s`;
		}
		return `${Math.round(bps / 1_000)} kb/s`;
	}
</script>

<div class="space-y-3">
	{#if status === 'loading'}
		<div class="text-gray-alpha-600 text-[11px] tracking-wide uppercase">Analyzing source…</div>
	{:else if status === 'error'}
		<div class="space-y-1 text-[11px] tracking-wide text-ds-red-700 uppercase">
			<p>Failed to read metadata.</p>
			{#if error}
				<p class="text-gray-alpha-600 text-[10px] normal-case">
					{error}
				</p>
			{/if}
		</div>
	{:else if metadata}
		<div class="space-y-2">
			<div class="grid grid-cols-2 gap-2 text-[11px] tracking-wide uppercase">
				<div class="text-gray-alpha-600">Duration</div>
				<div>{formatDuration(metadata.duration)}</div>
				<div class="text-gray-alpha-600">Video Codec</div>
				<div>{display(metadata.videoCodec)}</div>
				<div class="text-gray-alpha-600">Resolution</div>
				<div>{display(metadata.resolution)}</div>
				<div class="text-gray-alpha-600">Audio Codec</div>
				<div>{display(metadata.audioCodec)}</div>
				<div class="text-gray-alpha-600">Bitrate</div>
				<div>{formatBitrate(metadata.bitrate)}</div>
			</div>
		</div>
	{:else}
		<div class="text-gray-alpha-600 text-[11px] tracking-wide uppercase">Metadata unavailable.</div>
	{/if}
</div>
