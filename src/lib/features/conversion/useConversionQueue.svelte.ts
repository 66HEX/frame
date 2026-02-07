import {
	setupConversionListeners,
	startConversion as startConversionService,
	pauseConversion,
	resumeConversion,
	cancelConversion
} from '$lib/services/conversion';
import { sendAppNotification } from '$lib/services/notifications';
import { FileStatus, type FileItem } from '$lib/types';
import { _ } from '$lib/i18n';
import { get } from 'svelte/store';

export interface ConversionCallbacks {
	onFilesUpdate: (updater: (files: FileItem[]) => FileItem[]) => void;
	onLogsUpdate: (updater: (logs: Record<string, string[]>) => Record<string, string[]>) => void;
	getFiles: () => FileItem[];
	getIsProcessing: () => boolean;
	setIsProcessing: (value: boolean) => void;
}

export function createConversionQueue(callbacks: ConversionCallbacks) {
	let unlistenPromise: Promise<() => void> | null = null;

	function setupListeners() {
		unlistenPromise = setupConversionListeners(
			(payload) => {
				callbacks.onFilesUpdate((files) =>
					files.map((f) => {
						if (f.id === payload.id) {
							const status = f.status === FileStatus.QUEUED ? FileStatus.CONVERTING : f.status;
							return { ...f, status, progress: payload.progress };
						}
						return f;
					})
				);
			},
			(payload) => {
				callbacks.onFilesUpdate((files) =>
					files.map((f) =>
						f.id === payload.id ? { ...f, status: FileStatus.COMPLETED, progress: 100 } : f
					)
				);
				checkAllDone();
			},
			(payload) => {
				callbacks.onFilesUpdate((files) =>
					files.map((f) =>
						f.id === payload.id
							? { ...f, status: FileStatus.ERROR, conversionError: payload.error }
							: f
					)
				);
				checkAllDone();
			},
			(payload) => {
				callbacks.onLogsUpdate((logs) => {
					const current = logs[payload.id] || [];
					return { ...logs, [payload.id]: [...current, payload.line] };
				});
			}
		);

		return () => {
			unlistenPromise?.then((unlisten) => unlisten());
		};
	}

	function checkAllDone() {
		const files = callbacks.getFiles();
		const allDone = files.every(
			(f) =>
				f.status === FileStatus.COMPLETED ||
				f.status === FileStatus.ERROR ||
				f.status === FileStatus.IDLE
		);

		if (allDone && callbacks.getIsProcessing()) {
			const completedCount = files.filter((f) => f.status === FileStatus.COMPLETED).length;
			const errorCount = files.filter((f) => f.status === FileStatus.ERROR).length;

			if (completedCount > 0 || errorCount > 0) {
				const t = get(_);
				sendAppNotification(
					t('notifications.conversionFinishedTitle'),
					t('notifications.conversionFinishedBody', {
						values: { count: completedCount, errors: errorCount }
					})
				);
			}
			callbacks.setIsProcessing(false);
		}
	}

	async function startConversion() {
		const files = callbacks.getFiles();
		const pendingFiles = files.filter(
			(f) =>
				f.isSelectedForConversion &&
				f.status !== FileStatus.CONVERTING &&
				f.status !== FileStatus.QUEUED
		);

		if (pendingFiles.length === 0) return;

		callbacks.setIsProcessing(true);

		callbacks.onLogsUpdate((logs) => {
			const newLogs = { ...logs };
			pendingFiles.forEach((f) => {
				if (!newLogs[f.id]) {
					newLogs[f.id] = [];
				}
			});
			return newLogs;
		});

		callbacks.onFilesUpdate((files) =>
			files.map((f) => {
				const isPending =
					f.isSelectedForConversion &&
					f.status !== FileStatus.CONVERTING &&
					f.status !== FileStatus.QUEUED;
				return isPending ? { ...f, status: FileStatus.QUEUED, progress: 0 } : f;
			})
		);

		for (const file of pendingFiles) {
			await startConversionService(file.id, file.path, file.config, file.outputName);
		}
	}

	async function handlePause(id: string) {
		try {
			await pauseConversion(id);
			callbacks.onFilesUpdate((files) =>
				files.map((f) => (f.id === id ? { ...f, status: FileStatus.PAUSED } : f))
			);
		} catch (error) {
			console.error('Failed to pause:', error);
		}
	}

	async function handleResume(id: string) {
		try {
			await resumeConversion(id);
			callbacks.onFilesUpdate((files) =>
				files.map((f) => (f.id === id ? { ...f, status: FileStatus.CONVERTING } : f))
			);
		} catch (error) {
			console.error('Failed to resume:', error);
		}
	}

	async function cancelTask(id: string) {
		try {
			await cancelConversion(id);
		} catch (e) {
			console.error('Failed to cancel conversion task', e);
		}
	}

	return {
		setupListeners,
		startConversion,
		handlePause,
		handleResume,
		cancelTask,
		checkAllDone
	};
}
