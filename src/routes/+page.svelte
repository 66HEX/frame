<script lang="ts">
	import { onMount } from 'svelte';
	import { v4 as uuidv4 } from 'uuid';
	import { open } from '@tauri-apps/plugin-dialog';
	import { stat } from '@tauri-apps/plugin-fs';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { scale, fade } from 'svelte/transition';

	import Titlebar from '$lib/components/Titlebar.svelte';
	import LogsView from '$lib/components/LogsView.svelte';
	import FileList from '$lib/components/FileList.svelte';
	import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte';
	import EmptySelection from '$lib/components/EmptySelection.svelte';
	import AppSettingsSheet from '$lib/components/AppSettingsSheet.svelte';
	import PreviewPanel from '$lib/components/PreviewPanel.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';

	import {
		type FileItem,
		FileStatus,
		type ConversionConfig,
		type PresetDefinition,
		AUDIO_ONLY_CONTAINERS
	} from '$lib/types';
	import {
		startConversion as startConversionService,
		setupConversionListeners,
		pauseConversion,
		resumeConversion,
		cancelConversion
	} from '$lib/services/conversion';
	import { probeMedia, getDefaultAudioCodec } from '$lib/services/media';
	import {
		loadInitialMaxConcurrency,
		persistMaxConcurrency,
		loadAutoUpdateCheck
	} from '$lib/services/settings';
	import {
		DEFAULT_PRESETS,
		loadCustomPresets,
		saveCustomPresets,
		createCustomPreset,
		cloneConfig as clonePresetConfig,
		getDefaultConfig
	} from '$lib/services/presets';
	import { sendAppNotification } from '$lib/services/notifications';

	import { updateStore } from '$lib/stores/update.svelte';
	import { initCapabilities } from '$lib/stores/capabilities.svelte';
	import { checkForAppUpdate, installAppUpdate } from '$lib/services/update';
	import { marked } from 'marked';

	let files = $state<FileItem[]>([]);
	let selectedFileId = $state<string | null>(null);
	let isProcessing = $state(false);
	let customPresets = $state<PresetDefinition[]>([]);
	let maxConcurrencySetting = $state(2);
	let isDragging = $state(false);
	let showSettings = $state(false);

	let activeView = $state<'dashboard' | 'logs'>('dashboard');
	let logs = $state<Record<string, string[]>>({});

	let selectedFile = $derived(files.find((f) => f.id === selectedFileId));
	let selectedFileLocked = $derived(
		selectedFile
			? selectedFile.status === FileStatus.CONVERTING ||
				selectedFile.status === FileStatus.QUEUED ||
				selectedFile.status === FileStatus.COMPLETED
			: false
	);
	let totalSize = $derived(files.reduce((acc, curr) => acc + curr.size, 0));
	let presets = $derived([...DEFAULT_PRESETS, ...customPresets] as PresetDefinition[]);
	let selectedCount = $derived(files.filter((f) => f.isSelectedForConversion).length);

	onMount(() => {
		let unlistenDragDrop: (() => void) | undefined;
		let mounted = true;

		(async () => {
			await initCapabilities();
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
		const shouldCheck = await loadAutoUpdateCheck();
		if (!shouldCheck) return;

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

	function handleApplyPresetToAll(preset: PresetDefinition) {
		files = files.map((f) =>
			f.status === FileStatus.IDLE ? { ...f, config: clonePresetConfig(preset.config) } : f
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
			if (isProcessing) {
				const completedCount = files.filter((f) => f.status === FileStatus.COMPLETED).length;
				const errorCount = files.filter((f) => f.status === FileStatus.ERROR).length;

				if (completedCount > 0 || errorCount > 0) {
					sendAppNotification(
						$_('notifications.conversionFinishedTitle'),
						$_('notifications.conversionFinishedBody', {
							values: { count: completedCount, errors: errorCount }
						})
					);
				}
			}
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

	async function handleRemoveFile(id: string) {
		const file = files.find((f) => f.id === id);
		if (file && (file.status === FileStatus.CONVERTING || file.status === FileStatus.PAUSED)) {
			// If the file is currently processing, we must cancel it on the backend first
			// to kill the process and free up the queue slot.
			try {
				await cancelConversion(id);
			} catch (e) {
				console.error('Failed to cancel conversion task', e);
			}
		}

		files = files.filter((f) => f.id !== id);
		if (selectedFileId === id) selectedFileId = null;

		const newLogs = { ...logs };
		delete newLogs[id];
		logs = newLogs;

		checkAllDone();
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

	async function handleToggleBatch(id: string, isChecked: boolean) {
		files = files.map((f) => (f.id === id ? { ...f, isSelectedForConversion: isChecked } : f));
	}

	function handleToggleAllBatch(isChecked: boolean) {
		files = files.map((f) => ({ ...f, isSelectedForConversion: isChecked }));
	}

	async function handlePause(id: string) {
		try {
			await pauseConversion(id);
			files = files.map((f) => (f.id === id ? { ...f, status: FileStatus.PAUSED } : f));
		} catch (error) {
			console.error('Failed to pause:', error);
		}
	}

	async function handleResume(id: string) {
		try {
			await resumeConversion(id);
			files = files.map((f) => (f.id === id ? { ...f, status: FileStatus.CONVERTING } : f));
		} catch (error) {
			console.error('Failed to resume:', error);
		}
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
		onOpenSettings={() => (showSettings = !showSettings)}
	/>

	<div class="relative flex-1 overflow-hidden p-4">
		{#if activeView === 'dashboard'}
			<div class="grid h-full grid-cols-12 gap-4">
				<div class="col-span-12 h-full min-h-0 lg:col-span-8">
					<div class="grid h-full grid-rows-12 gap-4">
						<div class="row-span-8 min-h-0">
							{#if selectedFile}
								{#key selectedFile.id}
									<PreviewPanel
										filePath={selectedFile.path}
										initialStartTime={selectedFile.config.startTime}
										initialEndTime={selectedFile.config.endTime}
										rotation={selectedFile.config.rotation}
										flipHorizontal={selectedFile.config.flipHorizontal}
										flipVertical={selectedFile.config.flipVertical}
										onSave={handleSaveTrim}
										onUpdateConfig={updateSelectedConfig}
										initialCrop={selectedFile.config.crop}
										sourceWidth={selectedFile.metadata?.width}
										sourceHeight={selectedFile.metadata?.height}
										controlsDisabled={selectedFileLocked}
									/>
								{/key}
							{:else}
								<div
									class="flex h-full flex-col items-center justify-center rounded-lg border border-gray-alpha-100 bg-gray-alpha-100"
								></div>
							{/if}
						</div>

						<div class="row-span-4 min-h-0">
							<FileList
								{files}
								{selectedFileId}
								onSelect={(id) => (selectedFileId = id)}
								onRemove={handleRemoveFile}
								onToggleBatch={handleToggleBatch}
								onToggleAllBatch={handleToggleAllBatch}
								onPause={handlePause}
								onResume={handleResume}
							/>
						</div>
					</div>
				</div>

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
								onApplyPresetToAll={handleApplyPresetToAll}
								onSavePreset={handleSavePreset}
								onDeletePreset={handleDeletePreset}
								disabled={selectedFileLocked}
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
			transition:fade={{ duration: 100 }}
			class="absolute inset-0 z-100 flex items-center justify-center bg-background/60 backdrop-blur-sm"
		>
			<div
				transition:scale={{ start: 1.05, duration: 100, opacity: 1 }}
				class="flex h-36 w-72 flex-col items-center justify-center rounded-lg border border-dashed border-ds-blue-600 bg-ds-blue-900/20 shadow-2xl backdrop-blur-sm"
			>
				<p class="font-mono text-[10px] font-medium tracking-widest text-ds-blue-500 uppercase">
					{$_('fileList.importSource')}
				</p>
			</div>
		</div>
	{/if}

	{#if updateStore.showDialog}
		<div
			transition:fade={{ duration: 100 }}
			class="absolute inset-0 z-100 flex items-center justify-center bg-background/60 backdrop-blur-sm"
		>
			<div
				transition:scale={{ start: 1.05, duration: 100, opacity: 1 }}
				class="flex w-100 flex-col gap-4 rounded-lg border border-ds-blue-600 bg-ds-blue-900/20 p-3 shadow-2xl backdrop-blur-sm"
			>
				<div>
					<Label variant="section" class="text-foreground">{$_('update.available')}</Label>

					<p class="text-gray-alpha-600 font-mono text-[10px] font-medium tracking-wide uppercase">
						{$_('update.versionAvailable', { values: { version: updateStore.version } })}
					</p>
				</div>

				{#if updateStore.body}
					<div
						class="markdown-content text-gray-alpha-600 max-h-35 overflow-y-auto rounded bg-gray-alpha-100 p-3 text-xs tracking-wide uppercase"
					>
						<!-- eslint-disable-next-line svelte/no-at-html-tags -->
						{@html marked.parse(updateStore.body)}
					</div>
				{/if}

				{#if updateStore.error}
					<div class="text-xs text-ds-red-600">
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
						<Button variant="ghost" onclick={handleCancelUpdate}>{$_('update.later')}</Button>
						<Button onclick={handleUpdate}>{$_('update.updateNow')}</Button>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	{#if showSettings}
		<AppSettingsSheet
			maxConcurrency={maxConcurrencySetting}
			onUpdate={handleUpdateMaxConcurrency}
			onClose={() => (showSettings = false)}
		/>
	{/if}
</div>

<style>
	:global(.markdown-content h1),
	:global(.markdown-content h2),
	:global(.markdown-content h3) {
		font-size: 11px;
		font-weight: 500;
		color: var(--foreground);
		margin-top: 1em;
		margin-bottom: 0.5em;
	}

	:global(.markdown-content h1:first-child),
	:global(.markdown-content h2:first-child),
	:global(.markdown-content h3:first-child) {
		margin-top: 0;
	}

	:global(.markdown-content ul) {
		list-style-type: disc;
		padding-left: 1.5em;
		margin-bottom: 0.5em;
	}

	:global(.markdown-content li) {
		margin-bottom: 0.25em;
		font-size: 10px;
	}

	:global(.markdown-content p) {
		margin-bottom: 0.5em;
	}

	:global(.markdown-content strong) {
		font-weight: 600;
		color: var(--foreground);
	}
</style>
