<script lang="ts">
    import { v4 as uuidv4 } from "uuid";

    import Header from "$lib/components/Header.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import SettingsPanel from "$lib/components/SettingsPanel.svelte";
    import EmptySelection from "$lib/components/EmptySelection.svelte";
    import {
        type FileItem,
        FileStatus,
        type ConversionConfig,
    } from "$lib/types";

    const DEFAULT_CONFIG: ConversionConfig = {
        container: "mp4",
        videoCodec: "libx264",
        audioCodec: "aac",
        resolution: "original",
        crf: 23,
        preset: "medium",
    };

    let files = $state<FileItem[]>([]);
    let selectedFileId = $state<string | null>(null);
    let isProcessing = $state(false);

    let selectedFile = $derived(files.find((f) => f.id === selectedFileId));
    let totalSize = $derived(files.reduce((acc, curr) => acc + curr.size, 0));

    function handleAddFile(event: Event) {
        const target = event.target as HTMLInputElement;
        if (target.files) {
            const newFiles: FileItem[] = Array.from(target.files).map(
                (f: File) => ({
                    id: uuidv4(),
                    name: f.name,
                    size: f.size,
                    status: FileStatus.IDLE,
                    progress: 0,
                    originalFormat: f.name.split(".").pop() || "unknown",
                    config: { ...DEFAULT_CONFIG },
                    path: `/mock/path/to/${f.name}`,
                }),
            );
            files = [...files, ...newFiles];
            if (!selectedFileId && newFiles.length > 0) {
                selectedFileId = newFiles[0].id;
            }
            target.value = "";
        }
    }

    function handleRemoveFile(id: string) {
        files = files.filter((f) => f.id !== id);
        if (selectedFileId === id) selectedFileId = null;
    }

    function updateSelectedConfig(newConfig: Partial<ConversionConfig>) {
        if (selectedFileId) {
            files = files.map((f) =>
                f.id === selectedFileId
                    ? { ...f, config: { ...f.config, ...newConfig } }
                    : f,
            );
        }
    }

    function startConversion() {
        isProcessing = true;
        files = files.map((f) =>
            f.status === FileStatus.IDLE
                ? { ...f, status: FileStatus.CONVERTING, progress: 0 }
                : f,
        );
    }

    $effect(() => {
        if (!isProcessing) return;

        const interval = setInterval(() => {
            let allDone = true;
            const nextState = files.map((f) => {
                if (f.status === FileStatus.CONVERTING) {
                    const increment = Math.random() * 8 + 2;
                    const newProgress = Math.min(f.progress + increment, 100);
                    if (newProgress < 100) {
                        allDone = false;
                        return { ...f, progress: newProgress };
                    } else {
                        return {
                            ...f,
                            progress: 100,
                            status: FileStatus.COMPLETED,
                        };
                    }
                }
                return f;
            });

            files = nextState;

            if (allDone) isProcessing = false;
        }, 200);

        return () => clearInterval(interval);
    });
</script>

<div
    class="flex flex-col absolute inset-0 bg-black text-foreground font-sans overflow-hidden selection:bg-ds-blue-900 selection:text-white"
>
    <div class="flex-1 p-4 overflow-hidden">
        <div class="grid grid-cols-12 grid-rows-[auto_1fr] gap-4 h-full">
            <Header
                {totalSize}
                fileCount={files.length}
                {isProcessing}
                onAddFile={handleAddFile}
                onStartConversion={startConversion}
            />

            <FileList
                {files}
                {selectedFileId}
                onSelect={(id) => (selectedFileId = id)}
                onRemove={handleRemoveFile}
            />

            <div
                class="col-span-12 lg:col-span-4 border border-ds-gray-100 rounded-lg overflow-hidden flex flex-col"
            >
                {#if selectedFile}
                    <SettingsPanel
                        config={selectedFile.config}
                        onUpdate={updateSelectedConfig}
                        disabled={selectedFile.status ===
                            FileStatus.CONVERTING ||
                            selectedFile.status === FileStatus.COMPLETED}
                    />
                {:else}
                    <EmptySelection />
                {/if}
            </div>
        </div>
    </div>
</div>