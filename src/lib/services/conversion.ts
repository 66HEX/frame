import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ConversionConfig } from '../types';

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

export interface LogEvent {
	id: string;
	line: string;
}

export interface StartedEvent {
	id: string;
}

export async function startConversion(
	id: string,
	filePath: string,
	config: ConversionConfig,
	outputName?: string
) {
	try {
		await invoke('queue_conversion', {
			id,
			filePath,
			outputName,
			config
		});
	} catch (error) {
		console.error('Failed to queue conversion:', error);
		throw error;
	}
}

export async function pauseConversion(id: string) {
	try {
		await invoke('pause_conversion', { id });
	} catch (error) {
		console.error('Failed to pause conversion:', error);
		throw error;
	}
}

export async function resumeConversion(id: string) {
	try {
		await invoke('resume_conversion', { id });
	} catch (error) {
		console.error('Failed to resume conversion:', error);
		throw error;
	}
}

export async function cancelConversion(id: string) {
	try {
		await invoke('cancel_conversion', { id });
	} catch (error) {
		console.error('Failed to cancel conversion:', error);
		throw error;
	}
}

export async function setupConversionListeners(
	onProgress: (payload: ProgressEvent) => void,
	onCompleted: (payload: CompletedEvent) => void,
	onError: (payload: ErrorEvent) => void,
	onLog: (payload: LogEvent) => void,
	onStarted: (payload: StartedEvent) => void
): Promise<UnlistenFn> {
	const unlistenStarted = await listen<StartedEvent>('conversion-started', (event) => {
		onStarted(event.payload);
	});

	const unlistenProgress = await listen<ProgressEvent>('conversion-progress', (event) => {
		onProgress(event.payload);
	});

	const unlistenCompleted = await listen<CompletedEvent>('conversion-completed', (event) => {
		onCompleted(event.payload);
	});

	const unlistenError = await listen<ErrorEvent>('conversion-error', (event) => {
		onError(event.payload);
	});

	const unlistenLog = await listen<LogEvent>('conversion-log', (event) => {
		onLog(event.payload);
	});

	return () => {
		unlistenStarted();
		unlistenProgress();
		unlistenCompleted();
		unlistenError();
		unlistenLog();
	};
}
