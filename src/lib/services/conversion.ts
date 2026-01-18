import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ConversionConfig } from "../types";

export interface ProgressEvent {
    id: string;
    progress: number;
}

export interface CompletedEvent {
    id: string;
    outputPath: string;
}

export interface ErrorEvent {
    id: string;
    error: string;
}

export async function startConversion(
    id: string,
    filePath: string,
    config: ConversionConfig,
    outputName?: string,
) {
    try {
        await invoke("start_conversion", {
            id,
            filePath,
            outputName,
            config,
        });
    } catch (error) {
        console.error("Failed to start conversion:", error);
        throw error;
    }
}

export async function setupConversionListeners(
    onProgress: (payload: ProgressEvent) => void,
    onCompleted: (payload: CompletedEvent) => void,
    onError: (payload: ErrorEvent) => void
): Promise<UnlistenFn> {
    const unlistenProgress = await listen<ProgressEvent>("conversion-progress", (event) => {
        onProgress(event.payload);
    });

    const unlistenCompleted = await listen<CompletedEvent>("conversion-completed", (event) => {
        onCompleted(event.payload);
    });

    const unlistenError = await listen<ErrorEvent>("conversion-error", (event) => {
        onError(event.payload);
    });

    return () => {
        unlistenProgress();
        unlistenCompleted();
        unlistenError();
    };
}
