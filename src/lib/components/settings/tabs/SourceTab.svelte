<script lang="ts">
    import type { MetadataStatus, SourceMetadata } from "$lib/types";

    let {
        metadata,
        status = "idle",
        error,
    }: {
        metadata?: SourceMetadata;
        status?: MetadataStatus;
        error?: string;
    } = $props();

    function display(value?: string) {
        return value && value.trim().length > 0 ? value : "—";
    }
</script>

<div class="space-y-3">
    {#if status === "loading"}
        <div
            class="text-[11px] font-mono uppercase tracking-wide text-gray-alpha-600"
        >
            Analyzing source…
        </div>
    {:else if status === "error"}
        <div
            class="text-[11px] font-mono uppercase tracking-wide text-ds-red-700 space-y-1"
        >
            <p>Failed to read metadata.</p>
            {#if error}
                <p class="text-[10px] text-gray-alpha-600 normal-case">
                    {error}
                </p>
            {/if}
        </div>
    {:else if metadata}
        <div class="space-y-2">
            <div
                class="grid grid-cols-2 gap-2 text-[11px] font-mono uppercase tracking-wide"
            >
                <div class="text-gray-alpha-600">Duration</div>
                <div>{display(metadata.duration)}</div>
                <div class="text-gray-alpha-600">Video Codec</div>
                <div>{display(metadata.videoCodec)}</div>
                <div class="text-gray-alpha-600">Resolution</div>
                <div>{display(metadata.resolution)}</div>
                <div class="text-gray-alpha-600">Audio Codec</div>
                <div>{display(metadata.audioCodec)}</div>
                <div class="text-gray-alpha-600">Bitrate</div>
                <div>{display(metadata.bitrate)}</div>
            </div>
        </div>
    {:else}
        <div
            class="text-[11px] font-mono uppercase tracking-wide text-gray-alpha-600"
        >
            Metadata unavailable.
        </div>
    {/if}
</div>
