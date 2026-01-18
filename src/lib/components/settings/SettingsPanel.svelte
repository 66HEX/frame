<script lang="ts">
    import { Sliders } from "lucide-svelte";
    import type { ConversionConfig, PresetDefinition } from "$lib/types";

    import OutputTab from "./tabs/OutputTab.svelte";
    import PresetsTab from "./tabs/PresetsTab.svelte";
    import VideoTab from "./tabs/VideoTab.svelte";
    import AudioTab from "./tabs/AudioTab.svelte";

    const TABS = [
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
    } = $props();

    let activeTab = $state<TabId>("output");
</script>

<div class="flex flex-col h-full font-mono">
    <div
        class="h-10 border-b border-ds-gray-100 flex items-center justify-between px-4"
    >
        <div class="flex items-center gap-2">
            <Sliders size={12} class="text-ds-gray-500" />
            <span
                class="text-[10px] font-bold uppercase tracking-widest text-ds-gray-500"
                >Configuration</span
            >
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <div class="flex items-center gap-2 border-b border-ds-gray-100 pb-2">
            {#each TABS as tab}
                <button
                    class="px-3 py-1.5 text-[10px] font-mono uppercase tracking-widest rounded border transition-all
                    {activeTab === tab.id
                        ? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
                        : 'border-transparent text-ds-gray-500 hover:border-ds-gray-300'}"
                    onclick={() => (activeTab = tab.id)}
                >
                    {tab.label}
                </button>
            {/each}
        </div>

        {#if activeTab === "output"}
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
