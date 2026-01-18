<script lang="ts">
    import type { ConversionConfig } from "$lib/types";

    const RESOLUTIONS = ["original", "1080p", "720p", "480p"] as const;
    const VIDEO_CODECS = [
        { id: "libx264", label: "H.264 / AVC" },
        { id: "libx265", label: "H.265 / HEVC" },
        { id: "vp9", label: "VP9 / Web" },
        { id: "prores", label: "Apple ProRes" },
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

<div class="space-y-4">
    <div class="space-y-3">
        <span
            class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1"
        >
            Resolution
        </span>
        <div class="grid grid-cols-2 gap-2">
            {#each RESOLUTIONS as res}
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

    <div class="space-y-3">
        <span
            class="text-[10px] text-ds-gray-500 uppercase tracking-widest block border-b border-ds-gray-100 pb-1"
        >
            Video Encoder
        </span>
        <div class="grid grid-cols-1 gap-1.5">
            {#each VIDEO_CODECS as codec}
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

    <div class="space-y-2 pt-1">
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
        <div class="flex justify-between text-[9px] text-ds-gray-600 uppercase">
            <span>Lossless</span>
            <span>Smallest</span>
        </div>
    </div>
</div>
