<script lang="ts">
    import { onMount } from "svelte";
    import { v4 as uuidv4 } from "uuid";
    import { open } from "@tauri-apps/plugin-dialog";
    import { stat } from "@tauri-apps/plugin-fs";

    import Titlebar from "$lib/components/Titlebar.svelte";
    import LogsView from "$lib/components/LogsView.svelte";
    import FileList from "$lib/components/FileList.svelte";
    import SettingsPanel from "$lib/components/settings/SettingsPanel.svelte";
    import EmptySelection from "$lib/components/EmptySelection.svelte";
    import EstimatedOutputPanel from "$lib/components/EstimatedOutputPanel.svelte";
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
    import { probeMedia } from "$lib/services/media";
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

    let activeView = $state<"dashboard" | "logs">("dashboard");
    let logs = $state<Record<string, string[]>>({});

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
                files = files.map((f) => {
                    if (f.id === payload.id) {
                        const status =
                            f.status === FileStatus.QUEUED
                                ? FileStatus.CONVERTING
                                : f.status;
                        return { ...f, status, progress: payload.progress };
                    }
                    return f;
                });
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
            (payload) => {
                const current = logs[payload.id] || [];
                logs = { ...logs, [payload.id]: [...current, payload.line] };
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
                    metadataStatus: "idle",
                    path: pathStr,
                });
            }

            files = [...files, ...newFiles];
            for (const file of newFiles) {
                loadSourceMetadata(file.id, file.path);
            }
            if (!selectedFileId && newFiles.length > 0) {
                selectedFileId = newFiles[0].id;
            }
        }
    }

    function handleRemoveFile(id: string) {
        files = files.filter((f) => f.id !== id);
        if (selectedFileId === id) selectedFileId = null;

        const newLogs = { ...logs };
        delete newLogs[id];
        logs = newLogs;
    }

    function updateSelectedConfig(newConfig: Partial<ConversionConfig>) {
        if (selectedFileId) {
            files = files.map((f) => {
                if (f.id !== selectedFileId) return f;

                const nextConfig = { ...f.config, ...newConfig };

                if (
                    newConfig.container === "mp3" &&
                    nextConfig.audioCodec !== "mp3"
                ) {
                    nextConfig.audioCodec = "mp3";
                }

                return { ...f, config: nextConfig };
            });
        }
    }

    function updateSelectedOutputName(value: string) {
        if (selectedFileId) {
            files = files.map((f) =>
                f.id === selectedFileId ? { ...f, outputName: value } : f,
            );
        }
    }

    async function loadSourceMetadata(fileId: string, path: string) {
        files = files.map((f) =>
            f.id === fileId
                ? { ...f, metadataStatus: "loading", metadataError: undefined }
                : f,
        );
        try {
            const probeMetadata = await probeMedia(path);
            files = files.map((f) =>
                f.id === fileId
                    ? {
                          ...f,
                          metadataStatus: "ready",
                          metadata: probeMetadata,
                          metadataError: undefined,
                      }
                    : f,
            );
        } catch (error) {
            const message =
                error instanceof Error
                    ? error.message
                    : "Failed to probe source";
            files = files.map((f) =>
                f.id === fileId
                    ? {
                          ...f,
                          metadataStatus: "error",
                          metadataError: message,
                      }
                    : f,
            );
        }
    }

    async function startConversion() {
        const pendingFiles = files.filter((f) => f.status === FileStatus.IDLE);
        if (pendingFiles.length === 0) return;

        isProcessing = true;

        pendingFiles.forEach((f) => {
            if (!logs[f.id]) {
                logs = { ...logs, [f.id]: [] };
            }
        });

        files = files.map((f) =>
            f.status === FileStatus.IDLE
                ? { ...f, status: FileStatus.QUEUED, progress: 0 }
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
    class="flex flex-col absolute inset-0 text-foreground font-mono overflow-hidden"
>
    <Titlebar
        {totalSize}
        fileCount={files.length}
        {isProcessing}
        {activeView}
        onChangeView={(v) => (activeView = v)}
        onAddFile={handleAddFile}
        onStartConversion={startConversion}
    />

    <div class="flex-1 p-4 overflow-hidden relative">
        {#if activeView === "dashboard"}
            <div class="grid grid-cols-12 gap-4 h-full">
                <FileList
                    {files}
                    {selectedFileId}
                    onSelect={(id) => (selectedFileId = id)}
                    onRemove={handleRemoveFile}
                />

                <div
                    class="col-span-12 lg:col-span-4 grid gap-3 h-full grid-rows-[minmax(0,1fr)_160px] min-h-0"
                >
                    <div
                        class="border border-gray-alpha-100 rounded-lg bg-gray-alpha-100 overflow-y-auto h-full min-h-0 custom-scrollbar"
                    >
                        {#if selectedFile}
                            <SettingsPanel
                                config={selectedFile.config}
                                outputName={selectedFile.outputName}
                                metadata={selectedFile.metadata}
                                metadataStatus={selectedFile.metadataStatus}
                                metadataError={selectedFile.metadataError}
                                {presets}
                                onUpdate={updateSelectedConfig}
                                onUpdateOutputName={updateSelectedOutputName}
                                onApplyPreset={applyPresetToSelection}
                                onSavePreset={handleSavePreset}
                                onDeletePreset={handleDeletePreset}
                                disabled={selectedFile.status ===
                                    FileStatus.CONVERTING ||
                                    selectedFile.status ===
                                        FileStatus.COMPLETED}
                            />
                        {:else}
                            <EmptySelection />
                        {/if}
                    </div>

                    <div class="h-full">
                        <EstimatedOutputPanel
                            config={selectedFile?.config}
                            metadata={selectedFile?.metadata}
                            metadataStatus={selectedFile?.metadataStatus}
                        />
                    </div>
                </div>
            </div>
        {:else if activeView === "logs"}
            <LogsView {logs} {files} />
        {/if}
    </div>
</div>
