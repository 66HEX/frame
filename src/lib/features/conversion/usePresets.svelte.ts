import {
	DEFAULT_PRESETS,
	loadCustomPresets,
	saveCustomPresets,
	createCustomPreset,
	cloneConfig as clonePresetConfig
} from '$lib/services/presets';
import { FileStatus, type FileItem, type PresetDefinition } from '$lib/types';

export interface PresetsCallbacks {
	onFilesUpdate: (updater: (files: FileItem[]) => FileItem[]) => void;
	getSelectedFile: () => FileItem | undefined;
	getSelectedFileId: () => string | null;
}

export function createPresetsManager(callbacks: PresetsCallbacks) {
	let customPresets = $state<PresetDefinition[]>([]);

	const presets = $derived([...DEFAULT_PRESETS, ...customPresets] as PresetDefinition[]);

	async function loadPresets() {
		customPresets = await loadCustomPresets();
	}

	function applyPresetToSelection(preset: PresetDefinition) {
		const selectedFileId = callbacks.getSelectedFileId();
		if (!selectedFileId) return;

		const nextConfig = clonePresetConfig(preset.config);
		callbacks.onFilesUpdate((files) =>
			files.map((f) => (f.id === selectedFileId ? { ...f, config: nextConfig } : f))
		);
	}

	function handleApplyPresetToAll(preset: PresetDefinition) {
		callbacks.onFilesUpdate((files) =>
			files.map((f) =>
				f.status === FileStatus.IDLE ? { ...f, config: clonePresetConfig(preset.config) } : f
			)
		);
	}

	async function handleSavePreset(name: string): Promise<boolean> {
		const selectedFile = callbacks.getSelectedFile();
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

	return {
		get presets() {
			return presets;
		},
		get customPresets() {
			return customPresets;
		},
		loadPresets,
		applyPresetToSelection,
		handleApplyPresetToAll,
		handleSavePreset,
		handleDeletePreset
	};
}
