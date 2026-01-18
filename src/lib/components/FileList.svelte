<script lang="ts">
    import FileItemRow from "$lib/components/FileItemRow.svelte";
    import type { FileItem } from "$lib/types";

    let {
        files,
        selectedFileId,
        onSelect,
        onRemove,
    }: {
        files: FileItem[];
        selectedFileId: string | null;
        onSelect: (id: string) => void;
        onRemove: (id: string) => void;
    } = $props();
</script>

<div
    class="col-span-12 lg:col-span-8 border border-ds-gray-100 rounded-lg overflow-hidden flex flex-col relative group"
>
    <div
        class="h-10 border-b border-ds-gray-100 backdrop-blur-sm flex items-center px-4 z-10"
    >
        <div class="w-2.5 mr-4"></div>
        <div
            class="flex-1 grid grid-cols-12 gap-4 text-[10px] font-mono text-ds-gray-500 uppercase tracking-widest"
        >
            <div class="col-span-5">Name</div>
            <div class="col-span-3 text-right">Size</div>
            <div class="col-span-2 text-right">Target</div>
            <div class="col-span-2 text-right">State</div>
        </div>
        <div class="w-8 ml-4"></div>
    </div>

    <div class="flex-1 overflow-y-auto z-10 relative">
        {#if files.length === 0}
            <div class="h-full flex flex-col items-center justify-center p-12">
                <div class="text-center space-y-2 select-none">
                    <div
                        class="font-mono text-8xl md:text-9xl text-ds-gray-100 tracking-tighter leading-none"
                    >
                        00
                    </div>
                    <div
                        class="text-xl md:text-2xl font-mono text-ds-gray-600 tracking-tight"
                    >
                        FILES QUEUED
                    </div>
                </div>
            </div>
        {:else}
            <div>
                {#each files as file (file.id)}
                    <FileItemRow
                        item={file}
                        isSelected={selectedFileId === file.id}
                        onSelect={onSelect}
                        onRemove={onRemove}
                    />
                {/each}
                <div class="p-4 text-center border-t border-ds-gray-100 mt-2">
                    <span
                        class="text-[10px] font-mono text-ds-gray-600 uppercase tracking-widest"
                    >
                        END OF LIST // {files.length} OBJECTS
                    </span>
                </div>
            </div>
        {/if}
    </div>
</div>
