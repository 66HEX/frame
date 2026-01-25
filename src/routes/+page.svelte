<script lang="ts">
	import { onMount } from 'svelte';
	import { v4 as uuidv4 } from 'uuid';
	import { open } from '@tauri-apps/plugin-dialog';
	import { stat } from '@tauri-apps/plugin-fs';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';

	import Titlebar from '$lib/components/Titlebar.svelte';
	import LogsView from '$lib/components/LogsView.svelte';
	import FileList from '$lib/components/FileList.svelte';
	import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte';
	import EmptySelection from '$lib/components/EmptySelection.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';

	import {
		type FileItem,
		FileStatus,
		type ConversionConfig,
		type PresetDefinition
	} from '$lib/types';
	import {
		startConversion as startConversionService,
		setupConversionListeners
	} from '$lib/services/conversion';
	import { probeMedia } from '$lib/services/media';
	import { loadInitialMaxConcurrency, persistMaxConcurrency } from '$lib/services/settings';
	import {
		DEFAULT_PRESETS,
		loadCustomPresets,
		saveCustomPresets,
		createCustomPreset,
		cloneConfig as clonePresetConfig,
		getDefaultConfig
	} from '$lib/services/presets';

	import { updateStore } from '$lib/stores/update.svelte';
	import { checkForAppUpdate, installAppUpdate } from '$lib/services/update';

	let files = $state<FileItem[]>([]);
	let selectedFileId = $state<string | null>(null);
	let isProcessing = $state(false);
	let customPresets = $state<PresetDefinition[]>([]);
	let maxConcurrencySetting = $state(2);
	let isDragging = $state(false);

	let activeView = $state<'dashboard' | 'logs'>('dashboard');
	let logs = $state<Record<string, string[]>>({});

	let selectedFile = $derived(files.find((f) => f.id === selectedFileId));
	let totalSize = $derived(files.reduce((acc, curr) => acc + curr.size, 0));
	let presets = $derived([...DEFAULT_PRESETS, ...customPresets] as PresetDefinition[]);
	let selectedCount = $derived(files.filter((f) => f.isSelectedForConversion).length);

	onMount(() => {
		let unlistenDragDrop: (() => void) | undefined;
		let mounted = true;

		(async () => {
			customPresets = await loadCustomPresets();
			try {
				maxConcurrencySetting = await loadInitialMaxConcurrency();
			} catch (error) {
				console.error('Failed to load concurrency settings', error);
			}

			if (mounted) {
				const unlisten = await setupDragDrop();
				if (mounted) {
					unlistenDragDrop = unlisten;
				} else {
					unlisten();
				}
			}

			setTimeout(() => {
				if (mounted) invoke('close_splash');
			}, 1000);
		})();

		initUpdateCheck();

		return () => {
			mounted = false;
			if (unlistenDragDrop) {
				unlistenDragDrop();
			}
		};
	});

	async function setupDragDrop() {
		const unlistenEnter = await listen('tauri://drag-enter', () => {
			isDragging = true;
		});

		const unlistenLeave = await listen('tauri://drag-leave', () => {
			isDragging = false;
		});

		const unlistenDrop = await listen<{ paths: string[] }>('tauri://drag-drop', (event) => {
			isDragging = false;
			if (event.payload.paths && event.payload.paths.length > 0) {
				addFilesFromPaths(event.payload.paths);
			}
		});

		return () => {
			unlistenEnter();
			unlistenLeave();
			unlistenDrop();
		};
	}

	async function initUpdateCheck() {
		try {
			updateStore.isChecking = true;
			const result = await checkForAppUpdate();
			if (result.available) {
				updateStore.isAvailable = true;
				updateStore.version = result.version || '';
				updateStore.body = result.body || '';
				updateStore.updateObject = result.updateObject;
				updateStore.showDialog = true;
			}
		} catch (e) {
			console.error('Update check failed', e);
		} finally {
			updateStore.isChecking = false;
		}
	}

	async function handleUpdate() {
		if (!updateStore.updateObject) return;

		try {
			updateStore.isInstalling = true;
			updateStore.error = null;
			await installAppUpdate(updateStore.updateObject, (progress) => {
				updateStore.progress = progress;
			});
		} catch (e) {
			console.error('Update installation error:', e);
			updateStore.error = e instanceof Error ? e.message : String(e);
			updateStore.isInstalling = false;
		}
	}

	function handleCancelUpdate() {
		updateStore.showDialog = false;
	}

	function createInitialConfig(): ConversionConfig {
		return getDefaultConfig();
	}

	function deriveOutputName(fileName: string): string {
		const base = fileName.replace(/\.[^/.]+$/, '');
		return base ? `${base}_converted` : 'output_converted';
	}

	async function handleUpdateMaxConcurrency(value: number) {
		if (value < 1) return;

		try {
			await persistMaxConcurrency(value);
			maxConcurrencySetting = value;
		} catch (error) {
			console.error('Failed to persist max concurrency', error);
		}
	}

	function applyPresetToSelection(preset: PresetDefinition) {
		if (!selectedFileId) return;

		const nextConfig = clonePresetConfig(preset.config);
		files = files.map((f) => (f.id === selectedFileId ? { ...f, config: nextConfig } : f));
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
			console.error('Failed to persist preset', error);
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
			console.error('Failed to delete preset', error);
			customPresets = previous;
			return false;
		}
	}

	$effect(() => {
		const unlistenPromise = setupConversionListeners(
			(payload) => {
				files = files.map((f) => {
					if (f.id === payload.id) {
						const status = f.status === FileStatus.QUEUED ? FileStatus.CONVERTING : f.status;
						return { ...f, status, progress: payload.progress };
					}
					return f;
				});
			},
			(payload) => {
				files = files.map((f) =>
					f.id === payload.id ? { ...f, status: FileStatus.COMPLETED, progress: 100 } : f
				);
				checkAllDone();
			},
			(payload) => {
				files = files.map((f) => (f.id === payload.id ? { ...f, status: FileStatus.ERROR } : f));
				checkAllDone();
			},
			(payload) => {
				const current = logs[payload.id] || [];
				logs = { ...logs, [payload.id]: [...current, payload.line] };
			}
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
					f.status === FileStatus.IDLE
			)
		) {
			isProcessing = false;
		}
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
		const selected = await open({
			multiple: true,
			filters: [
				{
					name: 'Videos',
					extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm']
				}
			]
		});

		if (selected) {
			const paths = Array.isArray(selected) ? selected : [selected];
			await addFilesFromPaths(paths);
		}
	}

	function handleRemoveFile(id: string) {
		const file = files.find((f) => f.id === id);
		if (file && (file.status === FileStatus.CONVERTING || file.status === FileStatus.QUEUED)) {
			return;
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
			files = files.map((f) =>
				f.id === fileId
					? {
							...f,
							metadataStatus: 'ready',
							metadata: probeMetadata,
							metadataError: undefined
						}
					: f
			);
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

	async function handleToggleBatch(id: string, isChecked: boolean) {
		files = files.map((f) => (f.id === id ? { ...f, isSelectedForConversion: isChecked } : f));
	}

	function handleToggleAllBatch(isChecked: boolean) {
		files = files.map((f) => ({ ...f, isSelectedForConversion: isChecked }));
	}

	async function startConversion() {
		const pendingFiles = files.filter(
			(f) =>
				f.isSelectedForConversion &&
				f.status !== FileStatus.CONVERTING &&
				f.status !== FileStatus.QUEUED
		);

		if (pendingFiles.length === 0) return;

		isProcessing = true;

		pendingFiles.forEach((f) => {
			if (!logs[f.id]) {
				logs = { ...logs, [f.id]: [] };
			}
		});

		files = files.map((f) => {
			const isPending =
				f.isSelectedForConversion &&
				f.status !== FileStatus.CONVERTING &&
				f.status !== FileStatus.QUEUED;
			return isPending ? { ...f, status: FileStatus.QUEUED, progress: 0 } : f;
		});

		for (const file of pendingFiles) {
			await startConversionService(file.id, file.path, file.config, file.outputName);
		}
	}
</script>

<div class="absolute inset-0 flex flex-col overflow-hidden font-mono text-foreground">
	<Titlebar
		{totalSize}
		fileCount={files.length}
		{selectedCount}
		{isProcessing}
		{activeView}
		onChangeView={(v) => (activeView = v)}
		onAddFile={handleAddFile}
		onStartConversion={startConversion}
	/>

	<div class="relative flex-1 overflow-hidden p-4">
		{#if activeView === 'dashboard'}
			<div class="grid h-full grid-cols-12 gap-4">
				<FileList
					{files}
					{selectedFileId}
					onSelect={(id) => (selectedFileId = id)}
					onRemove={handleRemoveFile}
					onToggleBatch={handleToggleBatch}
					onToggleAllBatch={handleToggleAllBatch}
				/>

				<div class="col-span-12 h-full min-h-0 lg:col-span-4">
					<div
						class="custom-scrollbar h-full min-h-0 overflow-y-auto rounded-lg border border-gray-alpha-100 bg-gray-alpha-100"
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
								disabled={selectedFile.status === FileStatus.CONVERTING ||
									selectedFile.status === FileStatus.QUEUED ||
									selectedFile.status === FileStatus.COMPLETED}
								maxConcurrency={maxConcurrencySetting}
								onUpdateMaxConcurrency={handleUpdateMaxConcurrency}
							/>
						{:else}
							<EmptySelection />
						{/if}
					</div>
				</div>
			</div>
		{:else if activeView === 'logs'}
			<LogsView {logs} {files} />
		{/if}
	</div>

	{#if isDragging}
		<div
			class="absolute inset-0 z-50 flex items-center justify-center bg-background/60 backdrop-blur-sm"
		>
			<div
				class="flex flex-col items-center justify-center rounded-lg border border-dashed border-ds-blue-600 bg-ds-blue-900/20 px-6 py-3 shadow-2xl backdrop-blur-sm"
			>
				<p class="font-mono text-[10px] font-medium tracking-widest text-ds-blue-500 uppercase">
					Import Source Files
				</p>
			</div>
		</div>
	{/if}

	{#if updateStore.showDialog}
		<div
			class="absolute inset-0 z-50 flex items-center justify-center bg-background/60 backdrop-blur-sm"
		>
			<div
				class="flex w-100 flex-col gap-4 rounded-lg border border-ds-blue-600 bg-ds-blue-900/20 p-3 shadow-2xl backdrop-blur-sm"
			>
				<div>
					<Label variant="section" class="text-foreground">Update Available</Label>

					<p class="text-gray-alpha-600 font-mono text-[9px] font-medium tracking-widest uppercase">
						Version {updateStore.version} is available.
					</p>
				</div>

				{#if updateStore.body}
					<div
						class="text-gray-alpha-600 max-h-32 overflow-y-auto rounded-sm bg-gray-alpha-100 p-2 font-mono text-[10px]"
					>
						{updateStore.body}
					</div>
				{/if}

				{#if updateStore.error}
					<div class="text-[10px] text-ds-red-600">
						{updateStore.error}
					</div>
				{/if}

				{#if updateStore.isInstalling}
					<div class="space-y-1">
						<div class="bg-gray-alpha-200 h-1 w-full overflow-hidden rounded-full">
							<div
								class="h-full bg-ds-blue-600 transition-all duration-300"
								style="width: {updateStore.progress}%"
							></div>
						</div>
						<p class="text-gray-alpha-600 text-right text-[10px]">
							{Math.round(updateStore.progress)}%
						</p>
					</div>
				{:else}
					<div class="flex justify-end gap-2">
						<Button variant="ghost" onclick={handleCancelUpdate}>Later</Button>
						<Button onclick={handleUpdate}>Update Now</Button>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
