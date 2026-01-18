<script lang="ts">
    import { onMount } from "svelte";
    import { v4 as uuidv4 } from "uuid";
    import { open } from "@tauri-apps/plugin-dialog";
    import { stat } from "@tauri-apps/plugin-fs";

    import Titlebar from "$lib/components/Titlebar.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
    import EmptySelection from "$lib/components/EmptySelection.svelte";
    import {
        type FileItem,
        FileStatus,
        type ConversionConfig,
        type PresetDefinition,
    } from "$lib/types";
    import {
        startConversion as startConversionService,
        setupConversionListeners,
    } from "$lib/services/conversion";
    import {
        DEFAULT_PRESETS,
        loadCustomPresets,
        saveCustomPresets,
        createCustomPreset,
        cloneConfig as clonePresetConfig,
        getDefaultConfig,
    } from "$lib/services/presets";

    let files = $state<FileItem[]>([]);
    let selectedFileId = $state<string | null>(null);
    let isProcessing = $state(false);
    let customPresets = $state<PresetDefinition[]>([]);

    let selectedFile = $derived(files.find((f) => f.id === selectedFileId));
    let totalSize = $derived(files.reduce((acc, curr) => acc + curr.size, 0));
    let presets = $derived([
        ...DEFAULT_PRESETS,
        ...customPresets,
    ] as PresetDefinition[]);

    onMount(async () => {
        customPresets = await loadCustomPresets();
    });

    function createInitialConfig(): ConversionConfig {
        return getDefaultConfig();
    }

    function deriveOutputName(fileName: string): string {
        const base = fileName.replace(/\.[^/.]+$/, "");
        return base ? `${base}_converted` : "output_converted";
    }

    function applyPresetToSelection(preset: PresetDefinition) {
        if (!selectedFileId) return;

        const nextConfig = clonePresetConfig(preset.config);
        files = files.map((f) =>
            f.id === selectedFileId ? { ...f, config: nextConfig } : f,
        );
    }

    async function handleSavePreset(name: string): Promise<boolean> {
        if (!selectedFile) return false;
        const trimmedName = name.trim();
        if (!trimmedName) return false;

        const newPreset = createCustomPreset(trimmedName, selectedFile.config);
        const previous = customPresets;
        const updated = [...customPresets, newPreset];
        customPresets = updated;

        try {
            await saveCustomPresets(updated);
            return true;
        } catch (error) {
            console.error("Failed to persist preset", error);
            customPresets = previous;
            return false;
        }
    }

    async function handleDeletePreset(id: string): Promise<boolean> {
        const target = customPresets.find((p) => p.id === id);
        if (!target) return false;

        const previous = customPresets;
        const updated = customPresets.filter((p) => p.id !== id);
        customPresets = updated;

        try {
            await saveCustomPresets(updated);
            return true;
        } catch (error) {
            console.error("Failed to delete preset", error);
            customPresets = previous;
            return false;
        }
    }


    $effect(() => {
        const unlistenPromise = setupConversionListeners(
            (payload) => {
                files = files.map((f) =>
                    f.id === payload.id
                        ? { ...f, progress: payload.progress }
                        : f,
                );
            },
            (payload) => {
                files = files.map((f) =>
                    f.id === payload.id
                        ? { ...f, status: FileStatus.COMPLETED, progress: 100 }
                        : f,
                );
                checkAllDone();
            },
            (payload) => {
                files = files.map((f) =>
                    f.id === payload.id
                        ? { ...f, status: FileStatus.ERROR }
                        : f,
                );
                checkAllDone();
            },
        );

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    });

    function checkAllDone() {
        if (
            files.every(
                (f) =>
                    f.status === FileStatus.COMPLETED ||
                    f.status === FileStatus.ERROR ||
                    f.status === FileStatus.IDLE,
            )
        ) {
            isProcessing = false;
        }
    }

    async function handleAddFile() {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: "Videos",
                    extensions: ["mp4", "mov", "mkv", "avi", "webm"],
                },
            ],
        });

        if (selected) {
            const paths = Array.isArray(selected) ? selected : [selected];

            const newFiles: FileItem[] = [];

            for (const pathStr of paths) {
                const name = pathStr.split(/[/\\]/).pop() || "unknown";
                let size = 0;
                try {
                    const metadata = await stat(pathStr);
                    size = metadata.size;
                } catch (e) {
                    console.error("Failed to stat file:", pathStr, e);
                }

                newFiles.push({
                    id: uuidv4(),
                    name: name,
                    size: size,
                    status: FileStatus.IDLE,
                    progress: 0,
                    originalFormat: name.split(".").pop() || "unknown",
                    config: createInitialConfig(),
                    outputName: deriveOutputName(name),
                    path: pathStr,
                });
            }

            files = [...files, ...newFiles];
            if (!selectedFileId && newFiles.length > 0) {
                selectedFileId = newFiles[0].id;
            }
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

    function updateSelectedOutputName(value: string) {
        if (selectedFileId) {
            files = files.map((f) =>
                f.id === selectedFileId ? { ...f, outputName: value } : f,
            );
        }
    }

    async function startConversion() {
        const pendingFiles = files.filter((f) => f.status === FileStatus.IDLE);
        if (pendingFiles.length === 0) return;

        isProcessing = true;

        files = files.map((f) =>
            f.status === FileStatus.IDLE
                ? { ...f, status: FileStatus.CONVERTING, progress: 0 }
                : f,
        );

        for (const file of pendingFiles) {
            await startConversionService(
                file.id,
                file.path,
                file.config,
                file.outputName,
            );
        }
    }
</script>

<div
    class="flex flex-col absolute inset-0 text-foreground font-sans overflow-hidden selection:bg-ds-blue-900 selection:text-white"
>
    <Titlebar
        {totalSize}
        fileCount={files.length}
        {isProcessing}
        onAddFile={handleAddFile}
        onStartConversion={startConversion}
    />
    <div class="flex-1 p-4 overflow-hidden">
        <div class="grid grid-cols-12 gap-4 h-full">
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
                        outputName={selectedFile.outputName}
                        presets={presets}
                        onUpdate={updateSelectedConfig}
                        onUpdateOutputName={updateSelectedOutputName}
                        onApplyPreset={applyPresetToSelection}
                        onSavePreset={handleSavePreset}
                        onDeletePreset={handleDeletePreset}
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
