<script lang="ts">
	import type { MetadataStatus, SourceMetadata } from '$lib/types';
	import { _ } from '$lib/i18n';

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

		const timeMatch = raw.match(/^(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?$/);
		let secondsValue: number | null = null;

		if (timeMatch) {
			const [, hh, mm, ss, cs] = timeMatch;
			secondsValue =
				parseInt(hh, 10) * 3600 +
				parseInt(mm, 10) * 60 +
				parseInt(ss, 10) +
				(cs ? parseInt(cs, 10) / Math.pow(10, cs.length) : 0);
		} else {
			const numeric = Number(raw);
			if (Number.isFinite(numeric)) {
				secondsValue = numeric;
			}
		}

		if (secondsValue === null) return raw;

		const h = Math.floor(secondsValue / 3600);
		const m = Math.floor((secondsValue % 3600) / 60);
		const s = Math.floor(secondsValue % 60);

		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${pad(h)}:${pad(m)}:${pad(s)}`;
	}

	function formatResolution(meta?: SourceMetadata): string {
		if (meta?.width && meta?.height) {
			return `${meta.width}×${meta.height}`;
		}
		return meta?.resolution ?? '—';
	}

	function formatFrameRate(value?: number): string {
		if (!value || value <= 0) return '—';
		const formatted = value % 1 === 0 ? value.toFixed(0) : value.toFixed(3).replace(/\.?0+$/, '');
		return `${formatted} fps`;
	}

	function formatBitrateKbps(value?: number): string {
		if (!value || value <= 0) return '—';
		if (value >= 1000) {
			return `${(value / 1000).toFixed(2).replace(/\.?0+$/, '')} Mb/s`;
		}
		return `${Math.round(value)} kb/s`;
	}

	function formatContainerBitrate(raw?: string): string {
		if (!raw) return '—';
		const bps = Number(raw);
		if (!Number.isFinite(bps) || bps <= 0) {
			return raw;
		}
		if (bps >= 1_000_000) {
			return `${(bps / 1_000_000).toFixed(2).replace(/\.?0+$/, '')} Mb/s`;
		}
		return `${Math.round(bps / 1_000)} kb/s`;
	}
</script>

<div class="space-y-3">
	{#if status === 'loading'}
		<div class="text-gray-alpha-600 text-[11px] tracking-wide uppercase">
			{$_('source.analyzing')}
		</div>
	{:else if status === 'error'}
		<div class="space-y-1 text-[11px] tracking-wide text-ds-red-700 uppercase">
			<p>{$_('source.failedToRead')}</p>
			{#if error}
				<p class="text-gray-alpha-600 text-[10px] normal-case">
					{error}
				</p>
			{/if}
		</div>
	{:else if metadata}
		<div class="space-y-2">
			<div class="grid grid-cols-2 gap-2 text-[11px] tracking-wide uppercase">
				<div class="text-gray-alpha-600">{$_('source.duration')}</div>
				<div>{formatDuration(metadata.duration)}</div>

				<div class="text-gray-alpha-600">{$_('source.frameRate')}</div>
				<div>{formatFrameRate(metadata.frameRate)}</div>

				<div class="text-gray-alpha-600">{$_('source.dimensions')}</div>
				<div>{formatResolution(metadata)}</div>

				<div class="text-gray-alpha-600">{$_('source.videoCodec')}</div>
				<div>{display(metadata.videoCodec)}</div>

				<div class="text-gray-alpha-600">{$_('source.videoBitrate')}</div>
				<div>{formatBitrateKbps(metadata.videoBitrateKbps)}</div>

				<div class="text-gray-alpha-600">{$_('source.audioCodec')}</div>
				<div>{display(metadata.audioCodec)}</div>

				<div class="text-gray-alpha-600">{$_('source.containerBitrate')}</div>
				<div>{formatContainerBitrate(metadata.bitrate)}</div>
			</div>
		</div>
	{:else}
		<div class="text-gray-alpha-600 text-[11px] tracking-wide uppercase">
			{$_('source.unavailable')}
		</div>
	{/if}
</div>
