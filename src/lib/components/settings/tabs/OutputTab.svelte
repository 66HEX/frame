<script lang="ts">
	import {
		ALL_CONTAINERS,
		AUDIO_ONLY_CONTAINERS,
		type ConversionConfig,
		type SourceMetadata
	} from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';

	import { isAudioCodecAllowed, getDefaultAudioCodec } from '$lib/services/media';
	import {
		containerSupportsAudio,
		isAudioStreamCodecAllowedForContainer,
		isSubtitleCodecAllowedForContainer,
		isVideoStreamCodecAllowedForContainer
	} from '$lib/constants/media-rules';

	let {
		config,
		disabled = false,
		outputName = '',
		metadata,
		onUpdate,
		onUpdateOutputName
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		outputName?: string;
		metadata?: SourceMetadata;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		onUpdateOutputName?: (value: string) => void;
	} = $props();

	const isSourceAudioOnly = $derived(!!metadata && !metadata.videoCodec);
	const isCopyMode = $derived((config.processingMode ?? 'reencode') === 'copy');
	const selectedAudioCodecs = $derived.by(() => {
		const tracks = metadata?.audioTracks ?? [];
		if (tracks.length === 0) return [] as string[];
		const selectedAudioTracks = config.selectedAudioTracks ?? [];

		if (selectedAudioTracks.length === 0) {
			return tracks.map((track) => track.codec);
		}

		const selected = new Set(selectedAudioTracks);
		return tracks.filter((track) => selected.has(track.index)).map((track) => track.codec);
	});

	const selectedSubtitleCodecs = $derived.by(() => {
		const tracks = metadata?.subtitleTracks ?? [];
		if (tracks.length === 0) return [] as string[];
		const selectedSubtitleTracks = config.selectedSubtitleTracks ?? [];

		if (selectedSubtitleTracks.length === 0) {
			return tracks.map((track) => track.codec);
		}

		const selected = new Set(selectedSubtitleTracks);
		return tracks.filter((track) => selected.has(track.index)).map((track) => track.codec);
	});

	function sanitizeOutputName(value: string): string {
		const candidate = value.split(/[/\\]/).pop()?.trim() ?? '';
		return candidate === '.' || candidate === '..' ? '' : candidate;
	}

	function handleProcessingModeChange(mode: 'reencode' | 'copy') {
		onUpdate({ processingMode: mode });
	}

	function handleContainerChange(newContainer: string) {
		const updates: Partial<ConversionConfig> = { container: newContainer };

		if (
			!isCopyMode &&
			containerSupportsAudio(newContainer) &&
			!isAudioCodecAllowed(config.audioCodec, newContainer)
		) {
			updates.audioCodec = getDefaultAudioCodec(newContainer);
		}

		onUpdate(updates);
	}

	function isContainerCompatibleForStreamCopy(container: string): boolean {
		if (!isCopyMode) return true;
		if (container === 'gif') return false;
		if (!metadata) return true;

		const isAudioOnlyTarget = AUDIO_ONLY_CONTAINERS.includes(container);
		if (isAudioOnlyTarget) {
			return (
				selectedAudioCodecs.length > 0 &&
				selectedAudioCodecs.every((codec) =>
					isAudioStreamCodecAllowedForContainer(container, codec)
				)
			);
		}

		const videoCodec = metadata.videoCodec;
		if (!videoCodec) return false;
		if (!isVideoStreamCodecAllowedForContainer(container, videoCodec)) return false;

		if (
			selectedAudioCodecs.some((codec) => !isAudioStreamCodecAllowedForContainer(container, codec))
		) {
			return false;
		}

		if (
			selectedSubtitleCodecs.some((codec) => !isSubtitleCodecAllowedForContainer(container, codec))
		) {
			return false;
		}

		return true;
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('output.processingMode')}</Label>
		<div class="grid grid-cols-2 gap-2">
			<Button
				variant={!isCopyMode ? 'default' : 'secondary'}
				onclick={() => handleProcessingModeChange('reencode')}
				{disabled}
				class="w-full"
			>
				{$_('output.reencode')}
			</Button>
			<Button
				variant={isCopyMode ? 'default' : 'secondary'}
				onclick={() => handleProcessingModeChange('copy')}
				{disabled}
				class="w-full"
			>
				{$_('output.streamCopy')}
			</Button>
		</div>
		<p class="text-[10px] text-gray-alpha-600">
			{isCopyMode ? $_('output.streamCopyHint') : $_('output.reencodeHint')}
		</p>
	</div>

	<div class="space-y-3">
		<Label variant="section">{$_('output.outputName')}</Label>
		<Input
			type="text"
			value={outputName}
			oninput={(e) => onUpdateOutputName?.(sanitizeOutputName(e.currentTarget.value))}
			placeholder={$_('output.placeholder')}
			{disabled}
		/>
		<p class="text-[10px] text-gray-alpha-600">
			{$_('output.hint')}
		</p>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('output.container')}</Label>
		<div class="grid grid-cols-2 gap-2">
			{#each ALL_CONTAINERS as fmt (fmt)}
				{@const isVideoContainer = !AUDIO_ONLY_CONTAINERS.includes(fmt)}
				{@const isIncompatibleForCopy = !isContainerCompatibleForStreamCopy(fmt)}
				{@const isDisabled =
					disabled || (isSourceAudioOnly && isVideoContainer) || isIncompatibleForCopy}
				<Button
					variant={config.container === fmt ? 'default' : 'secondary'}
					onclick={() => handleContainerChange(fmt)}
					disabled={isDisabled}
					class="w-full uppercase"
				>
					{fmt}
				</Button>
			{/each}
		</div>
	</div>
</div>
