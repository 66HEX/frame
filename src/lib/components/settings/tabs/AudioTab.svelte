<script lang="ts">
    import type { ConversionConfig } from "$lib/types";

    const AUDIO_CODECS = [
        { id: "aac", label: "AAC / Stereo" },
        { id: "ac3", label: "Dolby Digital" },
        { id: "libopus", label: "Opus" },
        { id: "mp3", label: "MP3" },
    ] as const;

    let {
        config,
        disabled = false,
        onUpdate,
    }: {
        config: ConversionConfig;
        disabled?: boolean;
        onUpdate: (config: Partial<ConversionConfig>) => void;
    } = $props();
</script>

<div class="space-y-3">
    <span
        class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
    >
        Audio Codec
    </span>
    <div class="grid grid-cols-1">
        {#each AUDIO_CODECS as codec}
            <button
                onclick={() => onUpdate({ audioCodec: codec.id })}
                {disabled}
                class="text-[11px] py-1.5 px-3 border-l-2 text-left transition-all uppercase flex justify-between
                {config.audioCodec === codec.id
                    ? 'border-l-ds-blue-600 bg-gray-alpha-100 text-foreground pl-3'
                    : 'border-l-transparent text-gray-alpha-600 hover:text-foreground pl-2'}"
            >
                <span>{codec.id}</span>
                <span class="opacity-50 text-[9px]">{codec.label}</span>
            </button>
        {/each}
    </div>
</div>
