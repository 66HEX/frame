<script lang="ts">
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { Plus, Play, FileVideo, HardDrive } from "lucide-svelte";

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

    async function toggleMaximize() {
        const maximized = await appWindow.isMaximized();
        if (maximized) {
            await appWindow.unmaximize();
        } else {
            await appWindow.maximize();
        }
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
        <div class="flex items-center z-50 mr-2 group">
            <button
                onclick={close}
                class="size-6 rounded-full flex items-center justify-center transition-opacity"
                title="Close"
            >
                <svg
                    viewBox="-10 -10 20 20"
                    class="w-full h-full"
                    aria-hidden="true"
                >
                    <circle
                        r="6"
                        fill="#ff5f56"
                        stroke="#e0443e"
                        stroke-width="0.6"
                    />
                    <path
                        d="M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8"
                        stroke="#4a0002"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                    />
                </svg>
            </button>
            <button
                onclick={minimize}
                class="size-6 rounded-full flex items-center justify-center transition-opacity"
                title="Minimize"
            >
                <svg
                    viewBox="-10 -10 20 20"
                    class="w-full h-full"
                    aria-hidden="true"
                >
                    <circle
                        r="6"
                        fill="#ffbd2e"
                        stroke="#dea123"
                        stroke-width="0.6"
                    />
                    <line
                        x1="-2.4"
                        y1="0"
                        x2="2.4"
                        y2="0"
                        stroke="#5a3900"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                    />
                </svg>
            </button>
            <button
                onclick={toggleMaximize}
                class="size-6 rounded-full flex items-center justify-center transition-opacity"
                title="Toggle size"
            >
                <svg
                    viewBox="-10 -10 20 20"
                    class="w-full h-full"
                    aria-hidden="true"
                >
                    <circle
                        r="6"
                        fill="#27c93f"
                        stroke="#1aab29"
                        stroke-width="0.6"
                    />
                    <g
                        fill="#004200"
                        class="opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                    >
                        <path d="M-2.1 2.1 L-2.1 -1.5 L1.5 2.1 Z" />
                        <path d="M2.1 -2.1 L2.1 1.5 L-1.5 -2.1 Z" />
                    </g>
                </svg>
            </button>
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
                    ? 'bg-foreground text-black hover:bg-white border border-foreground opacity-50 cursor-not-allowed'
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
