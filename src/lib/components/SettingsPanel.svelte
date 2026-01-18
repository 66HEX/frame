<script lang="ts">
    import { onDestroy } from "svelte";
    import { Sliders, Trash2 } from "lucide-svelte";
    import type { ConversionConfig, PresetDefinition } from "../types";

    let {
        config,
        onUpdate,
        disabled,
        presets = [],
        onApplyPreset,
        onSavePreset,
        onDeletePreset,
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
    } = $props();

    let newPresetName = $state("");
    type NoticeTone = "success" | "error";
    let notice = $state<{ text: string; tone: NoticeTone } | null>(null);
    let noticeTimeout: ReturnType<typeof setTimeout> | null = null;

    onDestroy(() => {
        if (noticeTimeout) clearTimeout(noticeTimeout);
    });

    function configsMatch(a: ConversionConfig, b: ConversionConfig) {
        return (
            a.container === b.container &&
            a.videoCodec === b.videoCodec &&
            a.audioCodec === b.audioCodec &&
            a.resolution === b.resolution &&
            a.crf === b.crf &&
            a.preset === b.preset
        );
    }

    function showNotice(text: string, tone: NoticeTone = "success") {
        notice = { text, tone };
        if (noticeTimeout) clearTimeout(noticeTimeout);
        noticeTimeout = setTimeout(() => (notice = null), 2400);
    }

    async function savePreset() {
        if (!onSavePreset || disabled) return;
        if (!newPresetName.trim()) {
            showNotice("Name required", "error");
            return;
        }

        const result = await onSavePreset(newPresetName.trim());
        if (result === false) {
            showNotice("Preset not saved", "error");
            return;
        }

        newPresetName = "";
        showNotice("Preset saved");
    }

    function applyPreset(preset: PresetDefinition) {
        if (disabled) return;
        onApplyPreset?.(preset);
        showNotice(`Applied ${preset.name}`);
    }

    async function removePreset(preset: PresetDefinition) {
        if (!onDeletePreset || preset.builtIn) return;
        const result = await onDeletePreset(preset.id);
        if (result === false) {
            showNotice("Unable to delete", "error");
            return;
        }

        showNotice("Preset removed");
    }
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

    <div class="flex-1 overflow-y-auto p-4 space-y-6">
        <div class="space-y-3">
            <div
                class="flex items-center justify-between border-b border-ds-gray-100 pb-1"
            >
                <span
                    class="text-[10px] text-ds-gray-500 uppercase tracking-widest"
                    >Presets</span
                >
                {#if notice}
                    <span
                        class="text-[9px] font-mono uppercase tracking-wide {notice.tone ===
                        'error'
                            ? 'text-ds-red-700'
                            : 'text-ds-blue-600'}"
                    >
                        {notice.text}
                    </span>
                {/if}
            </div>

            <div class="flex gap-2">
                <input
                    type="text"
                    bind:value={newPresetName}
                    placeholder="Preset Label"
                    class="flex-1 text-[11px] font-mono uppercase tracking-wide px-3 py-1.5 border border-ds-gray-100 rounded bg-transparent focus:outline-none focus:border-ds-blue-600"
                    {disabled}
                />
                <button
                    onclick={savePreset}
                    disabled={disabled || !newPresetName.trim()}
                    class="px-3 py-1.5 text-[10px] font-mono uppercase tracking-wide border rounded transition-all
                    {disabled || !newPresetName.trim()
                        ? 'opacity-50 cursor-not-allowed border-ds-gray-200 text-ds-gray-400'
                        : 'border-ds-blue-600 text-ds-blue-600 hover:bg-ds-blue-900/10'}"
                >
                    Save
                </button>
            </div>

            <div class="space-y-1.5 max-h-36 overflow-y-auto pr-1">
                {#each presets as preset (preset.id)}
                    <div
                        class="flex items-center gap-2 border border-ds-gray-100 rounded px-2 py-1.5 group/preset-button"
                    >
                        <button
                            class="flex-1 text-left flex items-center justify-between gap-2 text-[11px] font-mono uppercase tracking-tight
                            {configsMatch(config, preset.config)
                                ? 'text-ds-blue-600'
                                : 'text-ds-gray-600 group-hover/preset-button:text-foreground'}"
                            onclick={() => applyPreset(preset)}
                            {disabled}
                        >
                            <span class="truncate">{preset.name}</span>
                            <span class="text-[9px] font-semibold">
                                {configsMatch(config, preset.config)
                                    ? "APPLIED"
                                    : "APPLY"}
                            </span>
                        </button>
                        {#if !preset.builtIn}
                            <button
                                class="size-4 flex items-center justify-center rounded text-ds-gray-500 hover:text-ds-red-700 transition-colors"
                                title="Delete preset"
                                onclick={() => removePreset(preset)}
                                {disabled}
                            >
                                <Trash2 size={12} />
                            </button>
                        {/if}
                    </div>
                {/each}
            </div>
        </div>

        <div class="space-y-2">
            <span
                class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1 mb-2"
            >
                Output Container
            </span>
            <div class="grid grid-cols-2 gap-2">
                {#each ["mp4", "mkv", "webm", "mov", "mp3"] as fmt}
                    <button
                        onclick={() => onUpdate({ container: fmt })}
                        {disabled}
                        class="text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase
                        {config.container === fmt
                            ? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
                            : 'bg-transparent text-ds-gray-500 border-ds-gray-200 hover:border-ds-gray-400'}"
                    >
                        {fmt}
                    </button>
                {/each}
            </div>
        </div>

        <div class="space-y-2">
            <span
                class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1 mb-2"
            >
                Resolution
            </span>
            <div class="grid grid-cols-2 gap-2">
                {#each ["original", "1080p", "720p", "480p"] as res}
                    <button
                        onclick={() => onUpdate({ resolution: res })}
                        {disabled}
                        class="text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase
                        {config.resolution === res
                            ? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
                            : 'bg-transparent text-ds-gray-500 border-ds-gray-200 hover:border-ds-gray-400'}"
                    >
                        {res}
                    </button>
                {/each}
            </div>
        </div>

        <div class="space-y-2">
            <span
                class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1 mb-2"
            >
                Video Encoder
            </span>
            <div class="grid grid-cols-1 gap-1.5">
                {#each [{ id: "libx264", label: "H.264 / AVC" }, { id: "libx265", label: "H.265 / HEVC" }, { id: "vp9", label: "VP9 / Web" }, { id: "prores", label: "Apple ProRes" }] as codec}
                    <button
                        onclick={() => onUpdate({ videoCodec: codec.id })}
                        {disabled}
                        class="text-[11px] py-1.5 px-3 border-l-2 text-left transition-all uppercase flex justify-between
                        {config.videoCodec === codec.id
                            ? 'border-l-ds-blue-600 bg-ds-gray-100/10 text-foreground pl-3'
                            : 'border-l-transparent text-ds-gray-500 hover:text-ds-gray-300 pl-2'}"
                    >
                        <span>{codec.id}</span>
                        <span class="opacity-50 text-[9px]">{codec.label}</span>
                    </button>
                {/each}
            </div>
        </div>

        <div class="space-y-2">
            <span
                class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1 mb-2"
            >
                Audio Codec
            </span>
            <div class="grid grid-cols-1 gap-1.5">
                {#each [{ id: "aac", label: "AAC / Stereo" }, { id: "ac3", label: "Dolby Digital" }, { id: "libopus", label: "Opus" }, { id: "mp3", label: "MP3" }] as codec}
                    <button
                        onclick={() => onUpdate({ audioCodec: codec.id })}
                        {disabled}
                        class="text-[11px] py-1.5 px-3 border-l-2 text-left transition-all uppercase flex justify-between
                        {config.audioCodec === codec.id
                            ? 'border-l-ds-blue-600 bg-ds-gray-100/10 text-foreground pl-3'
                            : 'border-l-transparent text-ds-gray-500 hover:text-ds-gray-300 pl-2'}"
                    >
                        <span>{codec.id}</span>
                        <span class="opacity-50 text-[9px]">{codec.label}</span>
                    </button>
                {/each}
            </div>
        </div>

        <div class="space-y-2 pt-2">
            <div class="flex justify-between items-end">
                <label
                    for="quality-factor"
                    class="text-[10px] text-ds-gray-500 uppercase tracking-widest"
                    >Quality Factor</label
                >
                <div
                    class="text-[10px] bg-ds-blue-900/20 text-ds-blue-600 border border-ds-blue-600 font-mono px-1.5 rounded font-bold"
                >
                    CRF {config.crf}
                </div>
            </div>
            <div class="h-1.5 bg-ds-gray-100 rounded-full w-full relative">
                <div
                    class="absolute top-0 left-0 h-full bg-foreground rounded-full"
                    style="width: {(config.crf / 51) * 100}%"
                ></div>
                <input
                    id="quality-factor"
                    type="range"
                    min="0"
                    max="51"
                    value={config.crf}
                    oninput={(e) =>
                        onUpdate({ crf: parseInt(e.currentTarget.value) })}
                    class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                    {disabled}
                />
            </div>
            <div
                class="flex justify-between text-[9px] text-ds-gray-600 uppercase"
            >
                <span>Lossless</span>
                <span>Smallest</span>
            </div>
        </div>
    </div>
</div>
