<script lang="ts">
    import { Plus, Play, FileVideo, HardDrive } from "lucide-svelte";

    let {
        totalSize,
        fileCount,
        isProcessing,
        onAddFile,
        onStartConversion,
    }: {
        totalSize: number;
        fileCount: number;
        isProcessing: boolean;
        onAddFile: (event: Event) => void;
        onStartConversion: () => void;
    } = $props();

    function formatTotalSize(bytes: number) {
        if (bytes === 0) return "0 KB";
        const mb = bytes / (1024 * 1024);
        return mb > 1000
            ? `${(mb / 1024).toFixed(2)} GB`
            : `${mb.toFixed(1)} MB`;
    }
</script>

<div
    class="col-span-12 h-16 border border-ds-gray-100 rounded-lg flex items-center justify-between px-6 shadow-sm"
>
    <div class="flex items-center gap-6">
        <div class="flex items-center gap-2">
            <div
                class="w-8 h-8 bg-foreground rounded flex items-center justify-center text-black"
            ></div>
            <div class="flex flex-col">
                <span class="text-sm font-bold tracking-tight"> Relay </span>
                <span class="text-[10px] font-mono text-ds-gray-500">
                    FFMPEG Converter
                </span>
            </div>
        </div>
        <div class="h-8 w-px bg-ds-gray-100"></div>
        <div class="flex items-center gap-4 text-xs font-mono text-ds-gray-500">
            <div class="flex items-center gap-2">
                <HardDrive size={14} />
                <span>STORAGE: {formatTotalSize(totalSize)}</span>
            </div>
            <div class="flex items-center gap-2">
                <FileVideo size={14} />
                <span>ITEMS: {fileCount}</span>
            </div>
        </div>
    </div>

    <div class="flex items-center gap-3">
        <div class="relative group">
            <input
                type="file"
                id="file-upload"
                multiple
                class="hidden"
                onchange={onAddFile}
            />
            <label
                for="file-upload"
                class="flex items-center gap-2 bg-ds-gray-100 hover:bg-ds-gray-200 text-foreground px-4 py-2 rounded text-xs font-mono font-medium transition-colors cursor-pointer border border-ds-gray-200 uppercase tracking-wide"
            >
                <Plus size={14} />
                Add Source
            </label>
        </div>

        <button
            onclick={onStartConversion}
            disabled={isProcessing || fileCount === 0}
            class="flex items-center gap-2 px-5 py-2 rounded text-xs font-mono font-medium uppercase tracking-wide transition-all
            {isProcessing || fileCount === 0
                ? 'bg-black border border-ds-gray-200 text-ds-gray-600 cursor-not-allowed'
                : 'bg-foreground text-black hover:bg-white border border-foreground'}"
        >
            {#if isProcessing}
                <span class="animate-pulse">PROCESSING...</span>
            {:else}
                <Play size={14} fill="currentColor" />
                START BATCH
            {/if}
        </button>
    </div>
</div>
