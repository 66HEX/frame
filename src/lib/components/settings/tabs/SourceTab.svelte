<script lang="ts">
	import type { MetadataStatus, SourceMetadata } from '$lib/types';
	import Label from '$lib/components/ui/Label.svelte';
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

	function display(value?: string | number) {
		const str = String(value);
		return value !== undefined && str.trim().length > 0 ? str : '—';
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

	function formatHz(value?: string): string {
		if (!value) return '—';
		const hz = parseInt(value, 10);
		if (isNaN(hz)) return value;
		if (hz >= 1000) {
			return `${(hz / 1000).toFixed(1).replace(/\.0$/, '')} kHz`;
		}
		return `${hz} Hz`;
	}
</script>

<div class="space-y-6">
	{#if status === 'loading'}
		<div class="text-[11px] tracking-wide text-gray-alpha-600">
			{$_('source.analyzing')}
		</div>
	{:else if status === 'error'}
		<div class="space-y-1 text-[11px] tracking-wide text-red-700">
			<p>{$_('source.failedToRead')}</p>
			{#if error}
				<p class="text-[10px] text-gray-alpha-600">
					{error}
				</p>
			{/if}
		</div>
	{:else if metadata}
		<div class="space-y-3">
			<Label variant="section">{$_('source.fileInfo')}</Label>
			<div class="grid grid-cols-2 gap-x-4 gap-y-2 text-[10px] tracking-wide">
				<div class="text-gray-alpha-600">{$_('source.duration')}</div>
				<div class="text-right font-medium">{formatDuration(metadata.duration)}</div>

				<div class="text-gray-alpha-600">{$_('source.containerBitrate')}</div>
				<div class="text-right font-medium">{formatContainerBitrate(metadata.bitrate)}</div>
			</div>
		</div>

		{#if metadata.videoCodec}
			<div class="space-y-3">
				<Label variant="section">{$_('source.videoStream')}</Label>
				<div class="grid grid-cols-2 gap-x-4 gap-y-2 text-[10px] tracking-wide">
					<div class="text-gray-alpha-600">{$_('source.videoCodec')}</div>
					<div class="text-right font-medium">{display(metadata.videoCodec)}</div>

					{#if metadata.profile}
						<div class="text-gray-alpha-600">{$_('source.profile')}</div>
						<div class="text-right font-medium">{display(metadata.profile)}</div>
					{/if}

					<div class="text-gray-alpha-600">{$_('source.dimensions')}</div>
					<div class="text-right font-medium">{formatResolution(metadata)}</div>

					<div class="text-gray-alpha-600">{$_('source.frameRate')}</div>
					<div class="text-right font-medium">{formatFrameRate(metadata.frameRate)}</div>

					{#if metadata.pixelFormat}
						<div class="text-gray-alpha-600">{$_('source.pixelFormat')}</div>
						<div class="text-right font-medium">{display(metadata.pixelFormat)}</div>
					{/if}

					{#if metadata.colorSpace}
						<div class="text-gray-alpha-600">{$_('source.colorSpace')}</div>
						<div class="text-right font-medium">{display(metadata.colorSpace)}</div>
					{/if}

					{#if metadata.colorRange}
						<div class="text-gray-alpha-600">{$_('source.colorRange')}</div>
						<div class="text-right font-medium">{display(metadata.colorRange)}</div>
					{/if}

					<div class="text-gray-alpha-600">{$_('source.videoBitrate')}</div>
					<div class="text-right font-medium">{formatBitrateKbps(metadata.videoBitrateKbps)}</div>
				</div>
			</div>
		{/if}

		{#if metadata.audioTracks && metadata.audioTracks.length > 0}
			<div class="space-y-3">
				<Label variant="section">{$_('source.audioStream')}</Label>
				<div class="space-y-4">
					{#each metadata.audioTracks as track, i (track.index)}
						<div class="space-y-2">
							<div class="flex items-center gap-2">
								<span class="text-[9px] font-bold tracking-widest text-blue-600"
									>{$_('source.track')} #{i + 1}</span
								>
								<div class="h-px flex-1 bg-gray-alpha-100"></div>
							</div>
							<div class="grid grid-cols-2 gap-x-4 gap-y-2 text-[10px] tracking-wide">
								<div class="text-gray-alpha-600">{$_('audio.codec')}</div>
								<div class="text-right font-medium">{display(track.codec)}</div>

								<div class="text-gray-alpha-600">{$_('audio.channels')}</div>
								<div class="text-right font-medium">{display(track.channels)}</div>

								{#if track.sampleRate}
									<div class="text-gray-alpha-600">{$_('source.sampleRate')}</div>
									<div class="text-right font-medium">{formatHz(track.sampleRate)}</div>
								{/if}

								{#if track.bitrateKbps}
									<div class="text-gray-alpha-600">{$_('source.bitrate')}</div>
									<div class="text-right font-medium">{formatBitrateKbps(track.bitrateKbps)}</div>
								{/if}

								{#if track.language}
									<div class="text-gray-alpha-600">{$_('source.language')}</div>
									<div class="text-right font-medium">{display(track.language)}</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{:else}
		<div class="text-[11px] tracking-wide text-gray-alpha-600">
			{$_('source.unavailable')}
		</div>
	{/if}
</div>
