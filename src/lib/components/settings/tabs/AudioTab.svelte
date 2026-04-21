<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, SourceMetadata } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import ListItem from '$lib/components/ui/ListItem.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import Slider from '$lib/components/ui/Slider.svelte';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';
	import {
		audioCodecSupportsVbr,
		getAudioQualityRange,
		isAudioCodecAllowed
	} from '$lib/services/media';
	import { capabilities } from '$lib/stores/capabilities.svelte';
	import { _ } from '$lib/i18n';

	type AudioCodecEntry = {
		id: string;
		label: string;
		requiresCapability?: keyof typeof capabilities.encoders;
	};

	const AUDIO_CODECS: AudioCodecEntry[] = [
		{ id: 'aac', label: 'AAC / Stereo' },
		{ id: 'libfdk_aac', label: 'AAC (Fraunhofer FDK)', requiresCapability: 'libfdk_aac' },
		{ id: 'ac3', label: 'Dolby Digital' },
		{ id: 'libopus', label: 'Opus' },
		{ id: 'mp3', label: 'MP3' },
		{ id: 'alac', label: 'ALAC (Lossless)' },
		{ id: 'flac', label: 'FLAC (Lossless)' },
		{ id: 'pcm_s16le', label: 'PCM / WAV' }
	];

	const CHANNELS = ['original', 'stereo', 'mono'] as const;

	let {
		config,
		disabled = false,
		copyMode = false,
		onUpdate,
		metadata
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		copyMode?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		metadata?: SourceMetadata;
	} = $props();

	const isLossless = $derived(['flac', 'alac', 'pcm_s16le'].includes(config.audioCodec));
	const encodeControlsDisabled = $derived(disabled || copyMode);
	const codecSupportsVbr = $derived(audioCodecSupportsVbr(config.audioCodec));
	const showVbrToggle = $derived(!isLossless && codecSupportsVbr);
	const isVbr = $derived(showVbrToggle && config.audioBitrateMode === 'vbr');
	const qualityRange = $derived(getAudioQualityRange(config.audioCodec));
	const qualityValue = $derived.by(() => {
		const parsed = Number.parseInt(config.audioQuality ?? '', 10);
		if (!qualityRange) return parsed;
		if (!Number.isFinite(parsed)) return qualityRange.defaultValue;
		return Math.min(qualityRange.max, Math.max(qualityRange.min, parsed));
	});

	function toggleTrack(index: number) {
		if (disabled) return;
		const current = config.selectedAudioTracks || [];
		if (current.includes(index)) {
			onUpdate({
				selectedAudioTracks: current.filter((i) => i !== index)
			});
		} else {
			onUpdate({ selectedAudioTracks: [...current, index] });
		}
	}

	function formatTrackBitrate(value?: number) {
		if (!value || value <= 0) {
			return null;
		}
		if (value >= 1000) {
			return `${(value / 1000).toFixed(2).replace(/\.?0+$/, '')} Mb/s`;
		}
		return `${Math.round(value)} kb/s`;
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('audio.channelsBitrate')}</Label>
		{#if copyMode}
			<p class="text-[10px] text-frame-gray-600">{$_('audio.copyModeHint')}</p>
		{/if}
		<div class="space-y-3">
			<div class="grid grid-cols-3 gap-2">
				{#each CHANNELS as ch (ch)}
					<Button
						variant={config.audioChannels === ch ? 'default' : 'secondary'}
						onclick={() => onUpdate({ audioChannels: ch })}
						disabled={encodeControlsDisabled}
						class="w-full"
					>
						{$_(`audio.${ch}`)}
					</Button>
				{/each}
			</div>

			{#if showVbrToggle}
				<div class="space-y-2 pt-1">
					<Label>{$_('audio.qualityControl')}</Label>
					<div class="grid grid-cols-2 gap-2">
						<Button
							variant={!isVbr ? 'default' : 'secondary'}
							onclick={() => onUpdate({ audioBitrateMode: 'bitrate' })}
							disabled={encodeControlsDisabled}
							class="w-full"
						>
							{$_('audio.targetBitrate')}
						</Button>
						<Button
							variant={isVbr ? 'default' : 'secondary'}
							onclick={() => onUpdate({ audioBitrateMode: 'vbr' })}
							disabled={encodeControlsDisabled}
							class="w-full"
						>
							{$_('audio.variableBitrate')}
						</Button>
					</div>
				</div>
			{/if}

			{#if isVbr && qualityRange}
				<div class="space-y-2 pt-1">
					<div class="flex items-end justify-between">
						<Label for="audio-quality">{$_('audio.qualityLevel')}</Label>
						<span
							class="button-highlight rounded bg-frame-gray-400 px-1.5 text-[10px] text-foreground shadow-sm"
							>Q {qualityValue}</span
						>
					</div>
					<Slider
						id="audio-quality"
						min={qualityRange.min}
						max={qualityRange.max}
						step={1}
						value={qualityValue}
						oninput={(e) => onUpdate({ audioQuality: String(e.currentTarget.value) })}
						disabled={encodeControlsDisabled}
					/>
					<div class="flex justify-between text-[10px] text-frame-gray-600">
						<span
							>{qualityRange.lowerIsBetter
								? $_('audio.qualityBest')
								: $_('audio.qualitySmallest')}</span
						>
						<span
							>{qualityRange.lowerIsBetter
								? $_('audio.qualitySmallest')
								: $_('audio.qualityBest')}</span
						>
					</div>
				</div>
			{:else}
				<div class="space-y-2 pt-1">
					<Label for="audio-bitrate">{$_('audio.bitrateKbps')}</Label>
					<Input
						id="audio-bitrate"
						type="text"
						inputmode="numeric"
						value={isLossless ? '' : config.audioBitrate}
						placeholder={isLossless ? $_('audio.bitrateIgnored') : ''}
						oninput={(e) => {
							const value = e.currentTarget.value.replace(/[^0-9]/g, '');
							onUpdate({ audioBitrate: value });
						}}
						disabled={encodeControlsDisabled || isLossless}
					/>
				</div>
			{/if}

			<div class="space-y-2 pt-1">
				<div class="flex items-center justify-between">
					<Label for="audio-volume">{$_('audio.volume')}</Label>
					<span
						class="button-highlight rounded bg-frame-gray-400 px-1.5 text-[10px] text-foreground shadow-sm"
						>{config.audioVolume}%</span
					>
				</div>
				<Slider
					id="audio-volume"
					min={0}
					max={200}
					step={1}
					value={config.audioVolume}
					oninput={(e) => onUpdate({ audioVolume: Number(e.currentTarget.value) })}
					disabled={encodeControlsDisabled}
				/>
				<div class="flex justify-between text-[10px] text-frame-gray-600">
					<span>{$_('audio.muted')}</span>
					<span>{$_('audio.maxVolume')}</span>
				</div>
			</div>

			<div class="flex items-start gap-2 pt-2">
				<Checkbox
					id="audio-normalize"
					checked={config.audioNormalize}
					onchange={(e) => onUpdate({ audioNormalize: e.currentTarget.checked })}
					disabled={encodeControlsDisabled}
				/>
				<div class="space-y-0.5">
					<Label for="audio-normalize">{$_('audio.normalize')}</Label>
					<p class="text-[10px] text-frame-gray-600">
						{$_('audio.normalizeHint')}
					</p>
				</div>
			</div>
		</div>
	</div>
	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('audio.codec')}</Label>
		<div class="grid grid-cols-1">
			{#each AUDIO_CODECS as codec (codec.id)}
				{@const capabilityOk =
					!codec.requiresCapability || capabilities.encoders[codec.requiresCapability]}
				{#if capabilityOk}
					{@const allowed = isAudioCodecAllowed(codec.id, config.container)}
					<ListItem
						selected={config.audioCodec === codec.id}
						onclick={() => onUpdate({ audioCodec: codec.id })}
						disabled={encodeControlsDisabled || !allowed}
						class={cn((encodeControlsDisabled || !allowed) && 'pointer-events-none opacity-50')}
					>
						<span>{codec.id.toUpperCase()}</span>
						<span class="text-[10px] opacity-50">
							{!allowed ? $_('audio.incompatibleContainer') : codec.label}
						</span>
					</ListItem>
				{/if}
			{/each}
		</div>
	</div>

	{#if metadata?.audioTracks && metadata.audioTracks.length > 0}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('audio.sourceTracks')}</Label>
			<div class="grid grid-cols-1 gap-2">
				{#each metadata.audioTracks as track (track.index)}
					{@const isSelected = (config.selectedAudioTracks || []).includes(track.index)}
					{@const trackBitrate = formatTrackBitrate(track.bitrateKbps)}
					<Button
						variant={isSelected ? 'default' : 'secondary'}
						onclick={() => toggleTrack(track.index)}
						{disabled}
						class="flex h-auto w-full items-center justify-between px-3 py-2 text-left"
					>
						<div class="space-y-0.5">
							<div class="flex items-center gap-2">
								<span class="text-[10px] opacity-70">
									#{track.index}
								</span>
								<span class="text-[10px]">
									{track.codec}
								</span>
								<div class="text-[10px]">
									<span class="mx-0.5">•</span>

									{track.channels}
									{$_('audio.channels')}
									{#if track.language}
										<span class="mx-0.5">•</span>
										{track.language}{/if}
									{#if track.label}
										<span class="mx-0.5">•</span>
										{track.label}{/if}
									{#if trackBitrate}
										<span class="mx-0.5">•</span>
										{trackBitrate}
									{/if}
								</div>
							</div>
						</div>

						<div
							class={cn(
								'input-highlight flex h-3 w-3 items-center justify-center rounded-full bg-background transition-all'
							)}
						>
							<div
								class="h-1.5 w-1.5 rounded-full bg-frame-gray-600 transition-all"
								style="opacity: {isSelected ? 1 : 0}; transform: scale({isSelected ? 1 : 0.5});"
							></div>
						</div>
					</Button>
				{/each}
			</div>
		</div>
	{/if}
</div>
