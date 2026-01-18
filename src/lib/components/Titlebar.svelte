<script lang="ts">
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { X, Minus, Plus, Play, FileVideo, HardDrive } from "lucide-svelte";

    const appWindow = getCurrentWindow();

    let {
        totalSize = 0,
        fileCount = 0,
        isProcessing = false,
        onAddFile,
        onStartConversion,
    }: {
        totalSize?: number;
        fileCount?: number;
        isProcessing?: boolean;
        onAddFile?: () => void;
        onStartConversion?: () => void;
    } = $props();

    function minimize() {
        appWindow.minimize();
    }

    function close() {
        appWindow.close();
    }

    function formatTotalSize(bytes: number) {
        if (bytes === 0) return "0 KB";
        const mb = bytes / (1024 * 1024);
        return mb > 1000
            ? `${(mb / 1024).toFixed(2)} GB`
            : `${mb.toFixed(1)} MB`;
    }
</script>

<div
    class="w-full pt-2 flex items-center justify-between px-4 select-none z-50 shrink-0"
    data-tauri-drag-region
>
    <div class="flex items-center gap-6">
        <div class="flex items-center gap-2 z-50 mr-2 group">
            <button
                onclick={close}
                class="size-3.5 rounded-full bg-[#FF5F57] flex items-center justify-center transition-colors border border-black/10"
                title="Close"
            >
                <X
                    size={8}
                    class="opacity-0 group-hover:opacity-100 text-black/60"
                    strokeWidth={4}
                />
            </button>
            <button
                onclick={minimize}
                class="size-3.5 rounded-full bg-[#FEBC2E] flex items-center justify-center transition-colors border border-black/10"
                title="Minimize"
            >
                <Minus
                    size={8}
                    class="opacity-0 group-hover:opacity-100 text-black/60"
                    strokeWidth={4}
                />
            </button>
            <div
                class="size-3.5 rounded-full bg-[#28C840] border border-black/10 cursor-default"
            ></div>
        </div>

        <div class="h-6 w-px bg-ds-gray-100/50"></div>

        <div
            class="flex items-center pointer-events-none"
            data-tauri-drag-region
        >
            <span
                class="text-foreground font-bold tracking-tight uppercase text-xs"
                >Relay</span
            >
        </div>

        <div class="h-6 w-px bg-ds-gray-100/50"></div>

        <div
            class="flex items-center gap-4 text-[10px] font-mono text-ds-gray-500"
        >
            <div class="flex items-center gap-2">
                <HardDrive size={12} />
                <span>STORAGE: {formatTotalSize(totalSize)}</span>
            </div>
            <div class="flex items-center gap-2">
                <FileVideo size={12} />
                <span>ITEMS: {fileCount}</span>
            </div>
        </div>
    </div>

    <div class="flex items-center gap-3 mt-2">
        {#if onAddFile}
            <button
                onclick={onAddFile}
                class="flex items-center gap-2 bg-ds-gray-100 hover:bg-ds-gray-200 text-foreground px-3 py-1.5 rounded text-[10px] font-mono font-medium transition-colors cursor-pointer border border-ds-gray-200 uppercase tracking-wide"
            >
                <Plus size={12} />
                Add Source
            </button>
        {/if}

        {#if onStartConversion}
            <button
                onclick={onStartConversion}
                disabled={isProcessing || fileCount === 0}
                class="flex items-center gap-2 px-4 py-1.5 rounded text-[10px] font-mono font-medium uppercase tracking-wide transition-all
            {isProcessing || fileCount === 0
                    ? 'bg-black border border-ds-gray-200 text-ds-gray-600 cursor-not-allowed'
                    : 'bg-foreground text-black hover:bg-white border border-foreground'}"
            >
                {#if isProcessing}
                    <span class="animate-pulse">PROCESSING...</span>
                {:else}
                    <Play size={12} fill="currentColor" />
                    START
                {/if}
            </button>
        {/if}
    </div>
</div>
