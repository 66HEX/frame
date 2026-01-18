<script lang="ts">
    import type {
        ConversionConfig,
        OutputEstimate,
        SourceMetadata,
    } from "$lib/types";
    import { estimateOutput } from "$lib/services/media";

    const CONTAINERS = ["mp4", "mkv", "webm", "mov", "mp3"] as const;

    let {
        config,
        disabled = false,
        outputName = "",
        metadata,
        onUpdate,
        onUpdateOutputName,
    }: {
        config: ConversionConfig;
        disabled?: boolean;
        outputName?: string;
        metadata?: SourceMetadata;
        onUpdate: (config: Partial<ConversionConfig>) => void;
        onUpdateOutputName?: (value: string) => void;
    } = $props();

    let estimate = $state<OutputEstimate | null>(null);
    let estimating = $state(false);
    let estimateError = $state<string | null>(null);

    $effect(() => {
        estimating = true;
        estimateError = null;
        const currentConfig = JSON.parse(JSON.stringify(config));
        const currentMetadata = metadata
            ? JSON.parse(JSON.stringify(metadata))
            : undefined;
        let cancelled = false;

        estimateOutput(currentConfig, currentMetadata)
            .then((result) => {
                if (!cancelled) {
                    estimate = result;
                }
            })
            .catch((error) => {
                if (!cancelled) {
                    estimate = null;
                    estimateError =
                        error instanceof Error
                            ? error.message
                            : "Unable to estimate";
                }
            })
            .finally(() => {
                if (!cancelled) {
                    estimating = false;
                }
            });

        return () => {
            cancelled = true;
        };
    });

    function formatSize(sizeMb?: number) {
        if (!sizeMb) return "—";
        if (sizeMb >= 1024) {
            return `${(sizeMb / 1024).toFixed(1)} GB`;
        }
        return `${sizeMb.toFixed(1)} MB`;
    }
</script>

<div class="space-y-4">
    <div class="space-y-3">
        <div
            class="text-[10px] text-gray-alpha-600 uppercase tracking-widest border-b border-gray-alpha-100 pb-1"
        >
            Output Name
        </div>
        <input
            type="text"
            value={outputName}
            oninput={(e) => onUpdateOutputName?.(e.currentTarget.value)}
            placeholder="my_render_final"
            class="w-full text-[11px] font-mono uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-100 rounded bg-transparent focus:outline-none focus:border-ds-blue-600"
            {disabled}
        />
        <p class="text-[10px] text-gray-alpha-600 uppercase tracking-wide">
            Stored next to the original file. Extension follows the selected
            container automatically.
        </p>
    </div>

    <div class="space-y-3">
        <span
            class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
        >
            Output Container
        </span>
        <div class="grid grid-cols-2 gap-2">
            {#each CONTAINERS as fmt}
                <button
                    onclick={() => onUpdate({ container: fmt })}
                    {disabled}
                    class="text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase
                    {config.container === fmt
                        ? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
                        : 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'}"
                >
                    {fmt}
                </button>
            {/each}
        </div>
    </div>

    <div class="space-y-3">
        <span
            class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
        >
            Estimated Output
        </span>
        <div
            class="grid grid-cols-2 gap-2 text-[11px] font-mono uppercase tracking-wide"
        >
            <div class="text-gray-alpha-600">Size</div>
            <div class="text-foreground">
                {#if estimating}…{:else}{formatSize(estimate?.sizeMb)}{/if}
            </div>
            <div class="text-gray-alpha-600">Video</div>
            <div class="text-foreground">
                {estimate ? `${estimate.videoKbps} kb/s` : "—"}
            </div>
            <div class="text-gray-alpha-600">Audio</div>
            <div class="text-foreground">
                {estimate
                    ? estimate.audioKbps > 0
                        ? `${estimate.audioKbps} kb/s`
                        : "—"
                    : "—"}
            </div>
            <div class="text-gray-alpha-600">Total</div>
            <div class="text-foreground">
                {estimate ? `${estimate.totalKbps} kb/s` : "—"}
            </div>
        </div>
        {#if estimateError}
            <div class="text-[10px] text-ds-red-700 normal-case">
                {estimateError}
            </div>
        {/if}
    </div>
</div>
