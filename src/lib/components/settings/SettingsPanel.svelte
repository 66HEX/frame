<script lang="ts">
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

	const TABS = ['source', 'output', 'video', 'audio', 'presets'] as const;
	type TabId = (typeof TABS)[number];

	let {
		config,
		onUpdate,
		disabled,
		presets = [],
		onApplyPreset,
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
</script>

<div class="flex h-full flex-col">
	<div class="flex h-10 items-center justify-between border-b border-gray-alpha-100 px-4">
		<div class="flex w-full items-center justify-start gap-4">
			{#each TABS as tabId (tabId)}
				{@const isVideoDisabled =
					tabId === 'video' &&
					(AUDIO_ONLY_CONTAINERS.includes(config.container) || isSourceAudioOnly)}
				<button
					disabled={isVideoDisabled}
					class={cn(
						'text-[10px] font-medium tracking-widest uppercase transition-all',
						activeTab === tabId ? 'text-ds-blue-600' : 'text-gray-alpha-600 hover:text-foreground',
						isVideoDisabled && 'pointer-events-none opacity-50'
					)}
					onclick={() => (activeTab = tabId)}
				>
					{$_(`tabs.${tabId}`)}
				</button>
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
				{onSavePreset}
				{onDeletePreset}
			/>
		{:else if activeTab === 'video'}
			<VideoTab {config} {disabled} {onUpdate} />
		{:else}
			<AudioTab {config} {disabled} {onUpdate} {metadata} />
		{/if}
	</div>
</div>
