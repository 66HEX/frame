<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { scale, fade } from 'svelte/transition';

	import { Titlebar } from '$lib/components/layout';
	import { LogsView } from '$lib/components/logs';
	import { FileList, EmptySelection } from '$lib/components/file-list';
	import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte';
	import AppSettingsSheet from '$lib/components/AppSettingsSheet.svelte';
	import PreviewPanel from '$lib/components/PreviewPanel.svelte';
	import { _ } from '$lib/i18n';

	import { initCapabilities } from '$lib/stores/capabilities.svelte';
	import { loadInitialMaxConcurrency, persistMaxConcurrency } from '$lib/services/settings';

	import { createFileListManager, createDragDropManager } from '$lib/features/files';
	import { createConversionQueue, createPresetsManager } from '$lib/features/conversion';
	import { createAppUpdateManager, UpdateDialog } from '$lib/features/update';

	const fileListManager = createFileListManager();
	const dragDropManager = createDragDropManager({
		onFilesDropped: (paths) => fileListManager.addFilesFromPaths(paths)
	});
	const updateManager = createAppUpdateManager();

	const conversionQueue = createConversionQueue({
		onFilesUpdate: fileListManager.updateFiles,
		onLogsUpdate: fileListManager.updateLogs,
		getFiles: () => fileListManager.files,
		getIsProcessing: () => isProcessing,
		setIsProcessing: (value) => (isProcessing = value)
	});

	const presetsManager = createPresetsManager({
		onFilesUpdate: fileListManager.updateFiles,
		getSelectedFile: () => fileListManager.selectedFile,
		getSelectedFileId: () => fileListManager.selectedFileId
	});

	let isProcessing = $state(false);
	let maxConcurrencySetting = $state(2);
	let showSettings = $state(false);
	let activeView = $state<'dashboard' | 'logs'>('dashboard');

	const files = $derived(fileListManager.files);
	const selectedFile = $derived(fileListManager.selectedFile);
	const selectedFileLocked = $derived(fileListManager.selectedFileLocked);
	const totalSize = $derived(fileListManager.totalSize);
	const selectedCount = $derived(fileListManager.selectedCount);
	const logs = $derived(fileListManager.logs);
	const isDragging = $derived(dragDropManager.isDragging);
	const presets = $derived(presetsManager.presets);

	const hasActionableFiles = $derived(
		files.some((f) => f.isSelectedForConversion && f.status !== 'COMPLETED')
	);

	onMount(() => {
		let unlistenDragDrop: (() => void) | undefined;
		let mounted = true;

		(async () => {
			await initCapabilities();
			await presetsManager.loadPresets();

			try {
				maxConcurrencySetting = await loadInitialMaxConcurrency();
			} catch (error) {
				console.error('Failed to load concurrency settings', error);
			}

			if (mounted) {
				const unlisten = await dragDropManager.setupDragDrop();
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

		updateManager.initUpdateCheck();

		return () => {
			mounted = false;
			if (unlistenDragDrop) {
				unlistenDragDrop();
			}
		};
	});

	$effect(() => {
		const cleanup = conversionQueue.setupListeners();
		return cleanup;
	});

	// Error dialog handler - needs to stay here for i18n context
	$effect(() => {
		void fileListManager.files;
		// This effect set up separately to show error dialogs
	});

	async function handleUpdateMaxConcurrency(value: number) {
		if (value < 1) return;

		try {
			await persistMaxConcurrency(value);
			maxConcurrencySetting = value;
		} catch (error) {
			console.error('Failed to persist max concurrency', error);
		}
	}

	async function handleRemoveFile(id: string) {
		await fileListManager.handleRemoveFile(id, conversionQueue.cancelTask);
		conversionQueue.checkAllDone();
	}
</script>

<div class="absolute inset-0 flex flex-col overflow-hidden text-foreground">
	<Titlebar
		{totalSize}
		fileCount={files.length}
		{selectedCount}
		{isProcessing}
		{activeView}
		canStart={hasActionableFiles}
		onChangeView={(v) => (activeView = v)}
		onAddFile={fileListManager.handleAddFile}
		onStartConversion={conversionQueue.startConversion}
		onOpenSettings={() => (showSettings = !showSettings)}
	/>

	<div class="relative flex-1 overflow-hidden p-4">
		{#if activeView === 'dashboard'}
			<div class="grid h-full grid-cols-12 gap-4">
				<div class="col-span-8 h-full min-h-0">
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
										onSave={fileListManager.handleSaveTrim}
										onUpdateConfig={fileListManager.updateSelectedConfig}
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
								selectedFileId={fileListManager.selectedFileId}
								onSelect={(id) => fileListManager.selectFile(id)}
								onRemove={handleRemoveFile}
								onToggleBatch={fileListManager.handleToggleBatch}
								onToggleAllBatch={fileListManager.handleToggleAllBatch}
								onPause={conversionQueue.handlePause}
								onResume={conversionQueue.handleResume}
							/>
						</div>
					</div>
				</div>

				<div class="col-span-4 h-full min-h-0">
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
								onUpdate={fileListManager.updateSelectedConfig}
								onUpdateOutputName={fileListManager.updateSelectedOutputName}
								onApplyPreset={presetsManager.applyPresetToSelection}
								onApplyPresetToAll={presetsManager.handleApplyPresetToAll}
								onSavePreset={presetsManager.handleSavePreset}
								onDeletePreset={presetsManager.handleDeletePreset}
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
				class="flex h-36 w-72 flex-col items-center justify-center rounded-lg border border-dashed border-blue-600 bg-blue-900/20 shadow-2xl backdrop-blur-sm"
			>
				<p class="text-[10px] font-medium tracking-widest text-blue-500">
					{$_('fileList.importSource')}
				</p>
			</div>
		</div>
	{/if}

	<UpdateDialog onUpdate={updateManager.handleUpdate} onCancel={updateManager.handleCancelUpdate} />

	{#if showSettings}
		<AppSettingsSheet
			maxConcurrency={maxConcurrencySetting}
			onUpdate={handleUpdateMaxConcurrency}
			onClose={() => (showSettings = false)}
		/>
	{/if}
</div>
