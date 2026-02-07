import { v4 as uuidv4 } from 'uuid';
import { stat } from '@tauri-apps/plugin-fs';
import { probeMedia, getDefaultAudioCodec } from '$lib/services/media';
import { getDefaultConfig } from '$lib/services/presets';
import { cancelConversion } from '$lib/services/conversion';
import { openNativeFileDialog } from '$lib/services/dialog';
import {
	FileStatus,
	type FileItem,
	type ConversionConfig,
	AUDIO_ONLY_CONTAINERS
} from '$lib/types';

export interface FileListState {
	files: FileItem[];
	selectedFileId: string | null;
	logs: Record<string, string[]>;
}

export function createFileListManager() {
	let files = $state<FileItem[]>([]);
	let selectedFileId = $state<string | null>(null);
	let logs = $state<Record<string, string[]>>({});

	const selectedFile = $derived(files.find((f) => f.id === selectedFileId));
	const selectedFileLocked = $derived(
		selectedFile
			? selectedFile.status === FileStatus.CONVERTING ||
					selectedFile.status === FileStatus.QUEUED ||
					selectedFile.status === FileStatus.COMPLETED
			: false
	);
	const totalSize = $derived(files.reduce((acc, curr) => acc + curr.size, 0));
	const selectedCount = $derived(files.filter((f) => f.isSelectedForConversion).length);

	function createInitialConfig(): ConversionConfig {
		return getDefaultConfig();
	}

	function deriveOutputName(fileName: string): string {
		const base = fileName.replace(/\.[^/.]+$/, '');
		return base ? `${base}_converted` : 'output_converted';
	}

	async function addFilesFromPaths(paths: string[]) {
		const newFiles: FileItem[] = [];

		for (const pathStr of paths) {
			const name = pathStr.split(/[/\\]/).pop() || 'unknown';

			let size = 0;
			try {
				const metadata = await stat(pathStr);
				size = metadata.size;
			} catch (e) {
				console.error('Failed to stat file:', pathStr, e);
			}

			newFiles.push({
				id: uuidv4(),
				name: name,
				size: size,
				status: FileStatus.IDLE,
				progress: 0,
				originalFormat: name.split('.').pop() || 'unknown',
				config: createInitialConfig(),
				outputName: deriveOutputName(name),
				metadataStatus: 'idle',
				path: pathStr,
				isSelectedForConversion: true
			});
		}

		if (newFiles.length > 0) {
			files = [...files, ...newFiles];
			for (const file of newFiles) {
				loadSourceMetadata(file.id, file.path);
			}
			if (!selectedFileId) {
				selectedFileId = newFiles[0].id;
			}
		}
	}

	async function handleAddFile() {
		const selected = await openNativeFileDialog({
			multiple: true,
			filters: [
				{
					name: 'Media Files',
					extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'mp3', 'm4a', 'wav', 'flac']
				},
				{
					name: 'Videos',
					extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm']
				},
				{
					name: 'Audio',
					extensions: ['mp3', 'm4a', 'wav', 'flac']
				}
			]
		});

		if (selected) {
			const paths = Array.isArray(selected) ? selected : [selected];
			await addFilesFromPaths(paths);
		}
	}

	async function handleRemoveFile(id: string, cancelTask?: (id: string) => Promise<void>) {
		const file = files.find((f) => f.id === id);
		if (file && (file.status === FileStatus.CONVERTING || file.status === FileStatus.PAUSED)) {
			try {
				if (cancelTask) {
					await cancelTask(id);
				} else {
					await cancelConversion(id);
				}
			} catch (e) {
				console.error('Failed to cancel conversion task', e);
			}
		}

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

				if (newConfig.container === 'mp3' && nextConfig.audioCodec !== 'mp3') {
					nextConfig.audioCodec = 'mp3';
				}

				return { ...f, config: nextConfig };
			});
		}
	}

	function updateSelectedOutputName(value: string) {
		if (selectedFileId) {
			files = files.map((f) => (f.id === selectedFileId ? { ...f, outputName: value } : f));
		}
	}

	async function loadSourceMetadata(fileId: string, path: string) {
		files = files.map((f) =>
			f.id === fileId ? { ...f, metadataStatus: 'loading', metadataError: undefined } : f
		);
		try {
			const probeMetadata = await probeMedia(path);
			files = files.map((f) => {
				if (f.id !== fileId) return f;

				let newConfig = f.config;
				if (!probeMetadata.videoCodec && !AUDIO_ONLY_CONTAINERS.includes(f.config.container)) {
					const defaultAudioContainer = 'mp3';
					newConfig = {
						...f.config,
						container: defaultAudioContainer,
						audioCodec: getDefaultAudioCodec(defaultAudioContainer)
					};
				}

				return {
					...f,
					metadataStatus: 'ready',
					metadata: probeMetadata,
					metadataError: undefined,
					config: newConfig
				};
			});
		} catch (error) {
			const message = error instanceof Error ? error.message : 'Failed to probe source';
			files = files.map((f) =>
				f.id === fileId
					? {
							...f,
							metadataStatus: 'error',
							metadataError: message
						}
					: f
			);
		}
	}

	function handleToggleBatch(id: string, isChecked: boolean) {
		files = files.map((f) => (f.id === id ? { ...f, isSelectedForConversion: isChecked } : f));
	}

	function handleToggleAllBatch(isChecked: boolean) {
		files = files.map((f) => ({ ...f, isSelectedForConversion: isChecked }));
	}

	function handleSaveTrim(start?: string, end?: string) {
		if (!selectedFileId) return;
		files = files.map((f) => {
			if (f.id !== selectedFileId) return f;
			return {
				...f,
				config: {
					...f.config,
					startTime: start,
					endTime: end
				}
			};
		});
	}

	function selectFile(id: string | null) {
		selectedFileId = id;
	}

	function updateFiles(updater: (files: FileItem[]) => FileItem[]) {
		files = updater(files);
	}

	function updateLogs(updater: (logs: Record<string, string[]>) => Record<string, string[]>) {
		logs = updater(logs);
	}

	return {
		get files() {
			return files;
		},
		get selectedFileId() {
			return selectedFileId;
		},
		get selectedFile() {
			return selectedFile;
		},
		get selectedFileLocked() {
			return selectedFileLocked;
		},
		get totalSize() {
			return totalSize;
		},
		get selectedCount() {
			return selectedCount;
		},
		get logs() {
			return logs;
		},
		addFilesFromPaths,
		handleAddFile,
		handleRemoveFile,
		updateSelectedConfig,
		updateSelectedOutputName,
		handleToggleBatch,
		handleToggleAllBatch,
		handleSaveTrim,
		selectFile,
		updateFiles,
		updateLogs
	};
}
