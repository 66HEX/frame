<script lang="ts">
	import { untrack } from 'svelte';
	import { cn } from '$lib/utils/cn';
	import {
		AUDIO_ONLY_CONTAINERS,
		type ConversionConfig,
		type MetadataStatus,
		type PresetDefinition,
		type SourceMetadata
	} from '$lib/types';
	import { _ } from '$lib/i18n';

	import SourceTab from './tabs/SourceTab.svelte';
	import OutputTab from './tabs/OutputTab.svelte';
	import PresetsTab from './tabs/PresetsTab.svelte';
	import VideoTab from './tabs/VideoTab.svelte';
	import ImagesTab from './tabs/ImagesTab.svelte';
	import AudioTab from './tabs/AudioTab.svelte';
	import SubtitlesTab from './tabs/SubtitlesTab.svelte';
	import MetadataTab from './tabs/MetadataTab.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import {
		IconFileUp,
		IconFileDown,
		IconFilm,
		IconFileImage,
		IconMusic,
		IconCaptions,
		IconTags,
		IconBookmark
	} from '$lib/icons';
	import { containerSupportsAudio, containerSupportsSubtitles } from '$lib/constants/media-rules';

	const TABS = [
		'source',
		'output',
		'video',
		'images',
		'audio',
		'subtitles',
		'metadata',
		'presets'
	] as const;
	type TabId = (typeof TABS)[number];

	let {
		config,
		onUpdate,
		disabled,
		presets = [],
		onApplyPreset,
		onApplyPresetToAll,
		onSavePreset,
		onDeletePreset,
		outputName = '',
		onUpdateOutputName,
		metadata,
		metadataStatus = 'idle',
		metadataError
	}: {
		config: ConversionConfig;
		onUpdate: (newConfig: Partial<ConversionConfig>) => void;
		disabled: boolean;
		presets?: PresetDefinition[];
		onApplyPreset?: (preset: PresetDefinition) => void;
		onApplyPresetToAll?: (preset: PresetDefinition) => void;
		onSavePreset?: (name: string) => Promise<boolean | void> | boolean | void;
		onDeletePreset?: (id: string) => Promise<boolean | void> | boolean | void;
		outputName?: string;
		onUpdateOutputName?: (name: string) => void;
		metadata?: SourceMetadata;
		metadataStatus?: MetadataStatus;
		metadataError?: string;
	} = $props();

	let activeTab = $state<TabId>('source');

	const sourceKind = $derived(
		metadata?.mediaKind ?? (metadata && !metadata.videoCodec ? 'audio' : 'video')
	);
	const isSourceAudioOnly = $derived(sourceKind === 'audio');
	const isSourceImage = $derived(sourceKind === 'image');
	const isCopyMode = $derived((config.processingMode ?? 'reencode') === 'copy');
	const isAudioContainer = $derived(AUDIO_ONLY_CONTAINERS.includes(config.container));
	const supportsAudio = $derived(containerSupportsAudio(config.container) && !isSourceImage);
	const supportsSubtitles = $derived(
		!isSourceAudioOnly && !isSourceImage && containerSupportsSubtitles(config.container)
	);
	const supportsVideoTab = $derived(
		!isSourceAudioOnly && !isSourceImage && !isAudioContainer && !isCopyMode
	);
	const supportsImagesTab = $derived(isSourceImage && !isAudioContainer && !isCopyMode);
	const visibleTabs = $derived(
		TABS.filter((tabId) => {
			if (tabId === 'video') return supportsVideoTab;
			if (tabId === 'images') return supportsImagesTab;
			if (tabId === 'audio') return supportsAudio;
			if (tabId === 'subtitles') return supportsSubtitles;
			return true;
		})
	);

	$effect(() => {
		if (!visibleTabs.includes(activeTab)) {
			untrack(() => (activeTab = 'output'));
		}
	});

	const icons: Record<TabId, typeof IconFileUp> = {
		source: IconFileUp,
		output: IconFileDown,
		video: IconFilm,
		images: IconFileImage,
		audio: IconMusic,
		subtitles: IconCaptions,
		metadata: IconTags,
		presets: IconBookmark
	};
</script>

<div class="flex h-full flex-col">
	<div
		class="relative flex h-10 items-center justify-between px-4 after:pointer-events-none after:absolute after:inset-x-0 after:bottom-0 after:h-px after:bg-background after:shadow-2xs after:shadow-gray-alpha-100 after:content-['']"
	>
		<div class="flex w-full items-center justify-start gap-1">
			{#each visibleTabs as tabId (tabId)}
				{@const Icon = icons[tabId]}
				<Button
					variant={activeTab === tabId ? 'default' : 'ghost'}
					size="icon"
					title={$_(`tabs.${tabId}`)}
					class={cn('size-6')}
					onclick={() => (activeTab = tabId)}
				>
					<Icon size={16} />
				</Button>
			{/each}
		</div>
	</div>

	<div class="flex-1 space-y-4 overflow-y-auto p-4">
		{#if activeTab === 'source'}
			<SourceTab {metadata} status={metadataStatus} error={metadataError} />
		{:else if activeTab === 'output'}
			<OutputTab {config} {disabled} {metadata} {outputName} {onUpdate} {onUpdateOutputName} />
		{:else if activeTab === 'presets'}
			<PresetsTab
				{config}
				{disabled}
				{presets}
				{metadata}
				{onApplyPreset}
				{onApplyPresetToAll}
				{onSavePreset}
				{onDeletePreset}
			/>
		{:else if activeTab === 'video'}
			<VideoTab {config} disabled={disabled || !supportsVideoTab} {onUpdate} />
		{:else if activeTab === 'images'}
			<ImagesTab {config} disabled={disabled || !supportsImagesTab} {onUpdate} />
		{:else if activeTab === 'audio'}
			<AudioTab
				{config}
				copyMode={isCopyMode}
				disabled={disabled || !supportsAudio}
				{onUpdate}
				{metadata}
			/>
		{:else if activeTab === 'subtitles'}
			<SubtitlesTab
				{config}
				copyMode={isCopyMode}
				disabled={disabled || !supportsSubtitles}
				{onUpdate}
				{metadata}
			/>
		{:else}
			<MetadataTab {config} {disabled} {onUpdate} {metadata} />
		{/if}
	</div>
</div>
