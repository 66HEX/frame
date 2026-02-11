<script lang="ts">
	import { untrack } from 'svelte';
	import type { ConversionConfig } from '$lib/types';
	import { cn } from '$lib/utils/cn';
	import Button from '$lib/components/ui/Button.svelte';
	import ListItem from '$lib/components/ui/ListItem.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import Slider from '$lib/components/ui/Slider.svelte';
	import Checkbox from '$lib/components/ui/Checkbox.svelte';
	import { capabilities } from '$lib/stores/capabilities.svelte';
	import { _ } from '$lib/i18n';
	import {
		VIDEO_CODEC_OPTIONS,
		VIDEO_PRESETS,
		NVENC_ENCODERS,
		VIDEOTOOLBOX_ENCODERS,
		getFirstAllowedPreset,
		getFirstAllowedVideoCodec,
		isVideoCodecAllowed,
		isVideoPresetAllowed
	} from '$lib/services/video-compatibility';

	const RESOLUTIONS = ['original', '1080p', '720p', '480p', 'custom'] as const;

	const availableCodecs = $derived(
		VIDEO_CODEC_OPTIONS.filter((codec) => {
			if (codec.id === 'h264_videotoolbox') return capabilities.encoders.h264_videotoolbox;
			if (codec.id === 'h264_nvenc') return capabilities.encoders.h264_nvenc;
			if (codec.id === 'hevc_videotoolbox') return capabilities.encoders.hevc_videotoolbox;
			if (codec.id === 'hevc_nvenc') return capabilities.encoders.hevc_nvenc;
			if (codec.id === 'av1_nvenc') return capabilities.encoders.av1_nvenc;
			return true;
		})
	);

	const SCALING_ALGOS = ['bicubic', 'lanczos', 'bilinear', 'nearest'] as const;

	const ML_UPSCALING_OPTIONS = [
		{ id: 'none', label: 'None' },
		{ id: 'esrgan-2x', label: 'ESRGAN 2x' },
		{ id: 'esrgan-4x', label: 'ESRGAN 4x' }
	] as const;

	const FPS_OPTIONS = ['original', '24', '30', '60'] as const;

	let {
		config,
		disabled = false,
		onUpdate
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
	} = $props();

	const isNvencEncoder = $derived(NVENC_ENCODERS.has(config.videoCodec));
	const isVideotoolboxEncoder = $derived(VIDEOTOOLBOX_ENCODERS.has(config.videoCodec));
	const isHardwareEncoder = $derived(isNvencEncoder || isVideotoolboxEncoder);
	const mlUpscaleAvailable = $derived(capabilities.encoders.ml_upscale);
	const isMlUpscaleActive = $derived(config.mlUpscale && config.mlUpscale !== 'none');
	const effectiveResolution = $derived(isMlUpscaleActive ? 'original' : config.resolution);
	const presetOptions = VIDEO_PRESETS;

	$effect(() => {
		if (isMlUpscaleActive && config.resolution !== 'original') {
			untrack(() => onUpdate({ resolution: 'original' }));
		}
	});

	$effect(() => {
		if (!mlUpscaleAvailable && config.mlUpscale && config.mlUpscale !== 'none') {
			untrack(() => onUpdate({ mlUpscale: 'none' }));
		}
	});

	function firstAllowedCodec(container: string) {
		const fallbackId = getFirstAllowedVideoCodec(
			container,
			availableCodecs.map((codec) => codec.id)
		);
		return availableCodecs.find((codec) => codec.id === fallbackId);
	}

	$effect(() => {
		// We want to re-run this when container or videoCodec changes
		const container = config.container;
		const videoCodec = config.videoCodec;

		if (!isVideoCodecAllowed(container, videoCodec)) {
			const fallback = firstAllowedCodec(container);
			if (fallback) {
				untrack(() => onUpdate({ videoCodec: fallback.id }));
			}
		}
	});

	$effect(() => {
		// We want to re-run this when videoCodec or preset changes
		const videoCodec = config.videoCodec;
		const preset = config.preset;

		if (!isVideoPresetAllowed(videoCodec, preset)) {
			const fallback = getFirstAllowedPreset(videoCodec);
			if (fallback !== preset) {
				untrack(() => onUpdate({ preset: fallback }));
			}
		}
	});

	function toggleNvencOption(
		field: keyof Pick<ConversionConfig, 'nvencSpatialAq' | 'nvencTemporalAq'>
	) {
		if (disabled) return;
		onUpdate({ [field]: !config[field] } as Partial<ConversionConfig>);
	}

	function toggleVideotoolboxAllowSw() {
		if (disabled) return;
		onUpdate({ videotoolboxAllowSw: !config.videotoolboxAllowSw });
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('video.resolutionFramerate')}</Label>
		<div class="mb-2 grid grid-cols-2 gap-2">
			{#each RESOLUTIONS as res (res)}
				<Button
					variant={effectiveResolution === res ? 'selected' : 'outline'}
					onclick={() => onUpdate({ resolution: res })}
					disabled={disabled || isMlUpscaleActive}
					class="w-full"
				>
					{res}
				</Button>
			{/each}
		</div>

		{#if config.resolution === 'custom'}
			<div class="mb-2 grid grid-cols-2 gap-2 pt-1">
				<div class="flex flex-col gap-1">
					<Label for="width">{$_('video.width')}</Label>
					<Input
						id="width"
						type="text"
						inputmode="numeric"
						placeholder="1920"
						value={config.customWidth}
						oninput={(e) => {
							const value = e.currentTarget.value.replace(/[^0-9]/g, '');
							onUpdate({ customWidth: value });
						}}
						{disabled}
					/>
				</div>
				<div class="flex flex-col gap-1">
					<Label for="height">{$_('video.height')}</Label>
					<Input
						id="height"
						type="text"
						inputmode="numeric"
						placeholder="1080"
						value={config.customHeight}
						oninput={(e) => {
							const value = e.currentTarget.value.replace(/[^0-9]/g, '');
							onUpdate({ customHeight: value });
						}}
						{disabled}
					/>
				</div>
			</div>
		{/if}

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.scalingAlgorithm')}</Label>
			<div class="grid grid-cols-2 gap-2">
				{#each SCALING_ALGOS as algo (algo)}
					<Button
						variant={config.scalingAlgorithm === algo ? 'selected' : 'outline'}
						onclick={() => onUpdate({ scalingAlgorithm: algo })}
						disabled={disabled || config.resolution === 'original'}
						class="w-full"
					>
						{$_(`scalingAlgorithm.${algo}`)}
					</Button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.mlUpscaling')}</Label>
			<div class="grid grid-cols-2 gap-2">
				{#each ML_UPSCALING_OPTIONS as opt (opt.id)}
					<Button
						variant={(config.mlUpscale || 'none') === opt.id ? 'selected' : 'outline'}
						onclick={() => onUpdate({ mlUpscale: opt.id as ConversionConfig['mlUpscale'] })}
						disabled={disabled || (opt.id !== 'none' && !mlUpscaleAvailable)}
						class="w-full"
					>
						{opt.label}
					</Button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.framerate')}</Label>
			<div class="grid grid-cols-2 gap-2">
				{#each FPS_OPTIONS as opt (opt)}
					<Button
						variant={config.fps === opt ? 'selected' : 'outline'}
						onclick={() => onUpdate({ fps: opt })}
						{disabled}
						class="w-full"
					>
						{opt === 'original' ? $_('video.sameAsSource') : `${opt} fps`}
					</Button>
				{/each}
			</div>
		</div>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('video.encoder')}</Label>
		<div class="grid grid-cols-1">
			{#each availableCodecs as codec (codec.id)}
				{@const codecAllowed = isVideoCodecAllowed(config.container, codec.id)}
				<ListItem
					selected={codecAllowed && config.videoCodec === codec.id}
					onclick={() => codecAllowed && onUpdate({ videoCodec: codec.id })}
					disabled={disabled || !codecAllowed}
					class={cn(!codecAllowed && 'pointer-events-none opacity-50')}
				>
					<span>{codec.id}</span>
					<span class="text-[9px] opacity-50">
						{#if codecAllowed}
							{codec.label}
						{:else}
							{$_('video.codecIncompatible')}
						{/if}
					</span>
				</ListItem>
			{/each}
		</div>
	</div>

	{#if !isVideotoolboxEncoder}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.encodingSpeed')}</Label>
			<div class="grid grid-cols-1">
				{#each presetOptions as preset (preset)}
					{@const allowed = isVideoPresetAllowed(config.videoCodec, preset)}
					<ListItem
						selected={allowed && config.preset === preset}
						onclick={() => allowed && onUpdate({ preset })}
						disabled={disabled || !allowed}
						class={cn(!allowed && 'pointer-events-none opacity-50')}
					>
						<span>{$_(`encodingSpeed.${preset}`)}</span>
						<span class="text-[9px] opacity-50">
							{#if allowed}
								{$_(`encodingSpeed.${preset}Desc`)}
							{:else}
								{$_('video.presetIncompatible')}
							{/if}
						</span>
					</ListItem>
				{/each}
			</div>
		</div>
	{/if}

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('video.qualityControl')}</Label>
		<div class="grid grid-cols-2 gap-2">
			<Button
				variant={config.videoBitrateMode === 'crf' ? 'selected' : 'outline'}
				onclick={() => onUpdate({ videoBitrateMode: 'crf' })}
				{disabled}
				class="w-full"
			>
				{$_('video.constantQuality')}
			</Button>
			<Button
				variant={config.videoBitrateMode === 'bitrate' ? 'selected' : 'outline'}
				onclick={() => onUpdate({ videoBitrateMode: 'bitrate' })}
				{disabled}
				class="w-full"
			>
				{$_('video.targetBitrate')}
			</Button>
		</div>
	</div>

	{#if config.videoBitrateMode === 'crf'}
		<div class="space-y-2 pt-2">
			<div class="flex items-end justify-between">
				<Label for="quality-factor">
					{#if isHardwareEncoder}
						{$_('video.encodingQuality')}
					{:else}
						{$_('video.qualityFactor')}
					{/if}
				</Label>
				<div
					class="rounded border border-blue-600 bg-blue-900/20 px-1.5 text-[10px] font-medium text-blue-600"
				>
					{#if isHardwareEncoder}
						Q {config.quality}
					{:else}
						CRF {config.crf}
					{/if}
				</div>
			</div>
			<div class="py-2">
				{#if isHardwareEncoder}
					<Slider
						id="quality-factor"
						min={1}
						max={100}
						step={1}
						value={config.quality}
						oninput={(e) => onUpdate({ quality: parseInt(e.currentTarget.value) })}
						{disabled}
					/>
				{:else}
					<Slider
						id="quality-factor"
						min={0}
						max={51}
						value={config.crf}
						oninput={(e) => onUpdate({ crf: parseInt(e.currentTarget.value) })}
						{disabled}
					/>
				{/if}
			</div>
			<div class="flex justify-between text-[9px] text-gray-alpha-600">
				{#if isHardwareEncoder}
					<span>{$_('video.lowQuality')}</span>
					<span>{$_('video.bestQuality')}</span>
				{:else}
					<span>{$_('video.lossless')}</span>
					<span>{$_('video.smallest')}</span>
				{/if}
			</div>
		</div>
	{:else}
		<div class="space-y-2 pt-1">
			<div class="flex items-end justify-between">
				<Label for="video-bitrate">{$_('video.targetBitrateKbps')}</Label>
			</div>
			<div class="flex items-center gap-2">
				<Input
					id="video-bitrate"
					type="text"
					inputmode="numeric"
					value={config.videoBitrate}
					oninput={(e) => {
						const value = e.currentTarget.value.replace(/[^0-9]/g, '');
						onUpdate({ videoBitrate: value });
					}}
					{disabled}
				/>
			</div>
		</div>
	{/if}

	{#if isNvencEncoder}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.nvencOptions')}</Label>
			<div class="space-y-2">
				<div class="flex items-start gap-2">
					<Checkbox
						id="nvenc-spatial-aq"
						checked={config.nvencSpatialAq}
						onchange={() => toggleNvencOption('nvencSpatialAq')}
						{disabled}
					/>
					<div class="space-y-0.5">
						<Label for="nvenc-spatial-aq">{$_('video.nvencSpatialAq')}</Label>
						<p class="text-[9px] text-gray-alpha-600">
							{$_('video.nvencSpatialAqHint')}
						</p>
					</div>
				</div>
				<div class="flex items-start gap-2">
					<Checkbox
						id="nvenc-temporal-aq"
						checked={config.nvencTemporalAq}
						onchange={() => toggleNvencOption('nvencTemporalAq')}
						{disabled}
					/>
					<div class="space-y-0.5">
						<Label for="nvenc-temporal-aq">{$_('video.nvencTemporalAq')}</Label>
						<p class="text-[9px] text-gray-alpha-600">
							{$_('video.nvencTemporalAqHint')}
						</p>
					</div>
				</div>
			</div>
		</div>
	{/if}

	{#if isVideotoolboxEncoder}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.videotoolboxOptions')}</Label>
			<div class="space-y-2">
				<div class="flex items-start gap-2">
					<Checkbox
						id="videotoolbox-allow-sw"
						checked={config.videotoolboxAllowSw}
						onchange={toggleVideotoolboxAllowSw}
						{disabled}
					/>
					<div class="space-y-0.5">
						<Label for="videotoolbox-allow-sw">{$_('video.videotoolboxAllowSw')}</Label>
						<p class="text-[9px] text-gray-alpha-600">
							{$_('video.videotoolboxAllowSwHint')}
						</p>
					</div>
				</div>
			</div>
		</div>
	{/if}

	{#if isHardwareEncoder}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('video.hardwareAcceleration')}</Label>
			<div class="flex items-start gap-2">
				<Checkbox
					id="hw-decode"
					checked={config.hwDecode}
					onchange={() => onUpdate({ hwDecode: !config.hwDecode })}
					{disabled}
				/>
				<div class="space-y-0.5">
					<Label for="hw-decode">{$_('video.hwDecode')}</Label>
					<p class="text-[9px] text-gray-alpha-600">
						{$_('video.hwDecodeHint')}
					</p>
				</div>
			</div>
		</div>
	{/if}
</div>
