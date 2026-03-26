<script lang="ts">
	import { untrack } from 'svelte';
	import type { ConversionConfig } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import ListItem from '$lib/components/ui/ListItem.svelte';
	import { cn } from '$lib/utils/cn';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';
	import { capabilities } from '$lib/stores/capabilities.svelte';
	import { isGifContainer } from '$lib/constants/media-rules';
	import {
		VIDEO_PIXEL_FORMAT_OPTIONS,
		getAllowedVideoPixelFormats,
		getFirstAllowedVideoPixelFormat
	} from '$lib/services/video-compatibility';

	const RESOLUTIONS = ['original', '1080p', '720p', '480p', 'custom'] as const;
	type ResolutionOption = (typeof RESOLUTIONS)[number];
	const SCALING_ALGOS = ['bicubic', 'lanczos', 'bilinear', 'nearest'] as const;
	const ML_UPSCALING_OPTIONS = [
		{ id: 'none', label: 'None' },
		{ id: 'esrgan-2x', label: 'ESRGAN 2x' },
		{ id: 'esrgan-4x', label: 'ESRGAN 4x' }
	] as const;

	let {
		config,
		disabled = false,
		onUpdate
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
	} = $props();

	const mlUpscaleAvailable = $derived(capabilities.encoders.ml_upscale);
	const isGifMode = $derived(isGifContainer(config.container));
	const isMlUpscaleActive = $derived(config.mlUpscale && config.mlUpscale !== 'none');
	const effectiveResolution = $derived(isMlUpscaleActive ? 'original' : config.resolution);
	const selectedPixelFormat = $derived(config.pixelFormat ?? 'auto');
	const allowedPixelFormats = $derived(
		new Set(getAllowedVideoPixelFormats(config.container, config.videoCodec))
	);

	$effect(() => {
		if (isMlUpscaleActive && config.resolution !== 'original') {
			untrack(() => onUpdate({ resolution: 'original' }));
		}
	});

	$effect(() => {
		if ((isGifMode || !mlUpscaleAvailable) && config.mlUpscale && config.mlUpscale !== 'none') {
			untrack(() => onUpdate({ mlUpscale: 'none' }));
		}
	});

	$effect(() => {
		const pixelFormat = config.pixelFormat ?? 'auto';
		const allowed = allowedPixelFormats;
		if (!allowed.has(pixelFormat)) {
			const fallback = getFirstAllowedVideoPixelFormat(
				config.container,
				config.videoCodec
			) as ConversionConfig['pixelFormat'];
			if (fallback !== pixelFormat) {
				untrack(() => onUpdate({ pixelFormat: fallback }));
			}
		}
	});

	function getResolutionLabel(resolution: ResolutionOption): string {
		if (resolution === 'original') return 'Original';
		if (resolution === 'custom') return 'Custom';
		return resolution;
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('video.resolutionFramerate')}</Label>
		<div class="mb-2 grid grid-cols-2 gap-2">
			{#each RESOLUTIONS as resolution (resolution)}
				<Button
					variant={effectiveResolution === resolution ? 'default' : 'secondary'}
					onclick={() => onUpdate({ resolution })}
					disabled={disabled || isMlUpscaleActive}
					class="w-full"
				>
					{getResolutionLabel(resolution)}
				</Button>
			{/each}
		</div>

		{#if config.resolution === 'custom'}
			<div class="mb-2 grid grid-cols-2 gap-2 pt-1">
				<div class="flex flex-col gap-1">
					<Label for="image-width">{$_('video.width')}</Label>
					<Input
						id="image-width"
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
					<Label for="image-height">{$_('video.height')}</Label>
					<Input
						id="image-height"
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
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('video.mlUpscaling')}</Label>
		<div class="grid grid-cols-2 gap-2">
			{#each ML_UPSCALING_OPTIONS as option (option.id)}
				<Button
					variant={(config.mlUpscale || 'none') === option.id ? 'default' : 'secondary'}
					onclick={() => onUpdate({ mlUpscale: option.id as ConversionConfig['mlUpscale'] })}
					disabled={disabled || (option.id !== 'none' && (!mlUpscaleAvailable || isGifMode))}
					class="w-full"
				>
					{option.label}
				</Button>
			{/each}
		</div>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('video.scalingAlgorithm')}</Label>
		<div class="grid grid-cols-2 gap-2">
			{#each SCALING_ALGOS as algorithm (algorithm)}
				<Button
					variant={config.scalingAlgorithm === algorithm ? 'default' : 'secondary'}
					onclick={() => onUpdate({ scalingAlgorithm: algorithm })}
					disabled={disabled || effectiveResolution === 'original'}
					class="w-full"
				>
					{$_(`scalingAlgorithm.${algorithm}`)}
				</Button>
			{/each}
		</div>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('source.pixelFormat')}</Label>
		<div class="grid grid-cols-1">
			{#each VIDEO_PIXEL_FORMAT_OPTIONS as option (option.id)}
				{@const allowed = allowedPixelFormats.has(option.id)}
				<ListItem
					selected={allowed && selectedPixelFormat === option.id}
					onclick={() =>
						allowed && onUpdate({ pixelFormat: option.id as ConversionConfig['pixelFormat'] })}
					disabled={disabled || !allowed}
					class={cn(!allowed && 'pointer-events-none opacity-50')}
				>
					<span>{option.label}</span>
					<span class="text-[10px] opacity-50">
						{#if option.id === 'auto'}
							Encoder default
						{:else if allowed}
							{option.id}
						{:else}
							{$_('video.codecIncompatible')}
						{/if}
					</span>
				</ListItem>
			{/each}
		</div>
	</div>
</div>
