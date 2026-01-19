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
    class="col-span-12 lg:col-span-8 border border-gray-alpha-100 bg-gray-alpha-100 rounded-lg overflow-hidden flex flex-col relative group"
>
    <div
        class="h-10 border-b border-gray-alpha-100 flex items-center px-4 z-10"
    >
        <div
            class="flex-1 grid grid-cols-12 gap-4 text-[10px] text-gray-alpha-600 font-medium uppercase tracking-widest"
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
            <div
                class="h-full flex flex-col items-center justify-center p-10 select-none"
            >
                <div
                    class="text-[10px] text-gray-alpha-600 font-medium uppercase"
                >
                    Drop files or use ADD SOURCE
                </div>
            </div>
        {:else}
            <div>
                {#each files as file (file.id)}
                    <FileItemRow
                        item={file}
                        isSelected={selectedFileId === file.id}
                        {onSelect}
                        {onRemove}
                    />
                {/each}
                <div
                    class="p-4 text-center border-t border-gray-alpha-100 mt-2"
                >
                    <span
                        class="text-[10px] text-gray-alpha-600 uppercase tracking-widest"
                    >
                        END OF LIST // {files.length} OBJECTS
                    </span>
                </div>
            </div>
        {/if}
    </div>
</div>
