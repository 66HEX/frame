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
	import AudioTab from './tabs/AudioTab.svelte';
	import SubtitlesTab from './tabs/SubtitlesTab.svelte';
	import MetadataTab from './tabs/MetadataTab.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import {
		IconFileUp,
		IconFileDown,
		IconFilm,
		IconMusic,
		IconCaptions,
		IconTags,
		IconBookmark
	} from '$lib/icons';
	import { containerSupportsAudio, containerSupportsSubtitles } from '$lib/constants/media-rules';

	const TABS = ['source', 'output', 'video', 'audio', 'subtitles', 'metadata', 'presets'] as const;
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

	const isSourceAudioOnly = $derived(!!metadata && !metadata.videoCodec);
	const isAudioContainer = $derived(AUDIO_ONLY_CONTAINERS.includes(config.container));
	const supportsAudio = $derived(containerSupportsAudio(config.container));
	const supportsSubtitles = $derived(
		!isSourceAudioOnly && containerSupportsSubtitles(config.container)
	);
	const supportsVideoTab = $derived(!isSourceAudioOnly && !isAudioContainer);

	$effect(() => {
		const tab = activeTab;
		if (tab === 'video' && !supportsVideoTab) {
			untrack(() => (activeTab = 'output'));
			return;
		}
		if (tab === 'audio' && !supportsAudio) {
			untrack(() => (activeTab = 'output'));
			return;
		}
		if (tab === 'subtitles' && !supportsSubtitles) {
			untrack(() => (activeTab = 'output'));
		}
	});

	const icons: Record<TabId, typeof IconFileUp> = {
		source: IconFileUp,
		output: IconFileDown,
		video: IconFilm,
		audio: IconMusic,
		subtitles: IconCaptions,
		metadata: IconTags,
		presets: IconBookmark
	};
</script>

<div class="flex h-full flex-col">
	<div class="flex h-10 items-center justify-between border-b border-gray-alpha-100 px-4">
		<div class="flex w-full items-center justify-start gap-1">
			{#each TABS as tabId (tabId)}
				{@const isVideoDisabled = tabId === 'video' && !supportsVideoTab}
				{@const isAudioDisabled = tabId === 'audio' && !supportsAudio}
				{@const isSubtitlesDisabled = tabId === 'subtitles' && !supportsSubtitles}
				{@const isDisabled = isVideoDisabled || isAudioDisabled || isSubtitlesDisabled}
				{@const Icon = icons[tabId]}
				<Button
					variant={activeTab === tabId ? 'selected' : 'ghost'}
					size="icon"
					title={$_(`tabs.${tabId}`)}
					class={cn('size-6 transition-all', isDisabled && 'pointer-events-none opacity-50')}
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
		{:else if activeTab === 'audio'}
			<AudioTab {config} disabled={disabled || !supportsAudio} {onUpdate} {metadata} />
		{:else if activeTab === 'subtitles'}
			<SubtitlesTab {config} disabled={disabled || !supportsSubtitles} {onUpdate} {metadata} />
		{:else}
			<MetadataTab {config} {disabled} {onUpdate} {metadata} />
		{/if}
	</div>
</div>
