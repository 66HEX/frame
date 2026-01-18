<script lang="ts">
    import { onDestroy } from "svelte";
    import { Trash2 } from "lucide-svelte";
    import type { ConversionConfig, PresetDefinition } from "$lib/types";

    let {
        config,
        presets = [],
        disabled = false,
        onApplyPreset,
        onSavePreset,
        onDeletePreset,
    }: {
        config: ConversionConfig;
        presets?: PresetDefinition[];
        disabled?: boolean;
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

<div class="space-y-3">
    <div
        class="flex items-center justify-between border-b border-ds-gray-100 pb-1"
    >
        <span class="text-[10px] text-ds-gray-500 uppercase tracking-widest"
            >Preset Library</span
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

    <div class="space-y-1.5 max-h-52 overflow-y-auto pr-1">
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
