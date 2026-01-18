<script lang="ts">
    import { Sliders } from "lucide-svelte";
    import type {
        ConversionConfig,
        MetadataStatus,
        PresetDefinition,
        SourceMetadata,
    } from "$lib/types";

    import SourceTab from "./tabs/SourceTab.svelte";
    import OutputTab from "./tabs/OutputTab.svelte";
    import PresetsTab from "./tabs/PresetsTab.svelte";
    import VideoTab from "./tabs/VideoTab.svelte";
    import AudioTab from "./tabs/AudioTab.svelte";

    const TABS = [
        { id: "source", label: "Source" },
        { id: "output", label: "Output" },
        { id: "presets", label: "Presets" },
        { id: "video", label: "Video" },
        { id: "audio", label: "Audio" },
    ] as const;
    type TabId = (typeof TABS)[number]["id"];

    let {
        config,
        onUpdate,
        disabled,
        presets = [],
        onApplyPreset,
        onSavePreset,
        onDeletePreset,
        outputName = "",
        onUpdateOutputName,
        metadata,
        metadataStatus = "idle",
        metadataError,
    }: {
        config: ConversionConfig;
        onUpdate: (newConfig: Partial<ConversionConfig>) => void;
        disabled: boolean;
        presets?: PresetDefinition[];
        onApplyPreset?: (preset: PresetDefinition) => void;
        onSavePreset?: (
            name: string,
        ) => Promise<boolean | void> | boolean | void;
        onDeletePreset?: (
            id: string,
        ) => Promise<boolean | void> | boolean | void;
        outputName?: string;
        onUpdateOutputName?: (name: string) => void;
        metadata?: SourceMetadata;
        metadataStatus?: MetadataStatus;
        metadataError?: string;
    } = $props();

    let activeTab = $state<TabId>("source");
</script>

<div class="flex flex-col h-full font-mono">
    <div
        class="h-10 border-b border-gray-alpha-100 flex items-center justify-between px-4"
    >
        <div class="flex items-center gap-4 w-full justify-start">
            {#each TABS as tab}
                <button
                    class="text-[10px] font-mono uppercase tracking-widest font-medium transition-all
                    {activeTab === tab.id
                        ? 'text-ds-blue-600'
                        : 'text-gray-alpha-600 hover:text-foreground'}"
                    onclick={() => (activeTab = tab.id)}
                >
                    {tab.label}
                </button>
            {/each}
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4">
        {#if activeTab === "source"}
            <SourceTab
                {metadata}
                status={metadataStatus}
                error={metadataError}
            />
        {:else if activeTab === "output"}
            <OutputTab
                {config}
                {disabled}
                {outputName}
                {onUpdate}
                {onUpdateOutputName}
            />
        {:else if activeTab === "presets"}
            <PresetsTab
                {config}
                {disabled}
                {presets}
                {onApplyPreset}
                {onSavePreset}
                {onDeletePreset}
            />
        {:else if activeTab === "video"}
            <VideoTab {config} {disabled} {onUpdate} />
        {:else}
            <AudioTab {config} {disabled} {onUpdate} />
        {/if}
    </div>
</div>
