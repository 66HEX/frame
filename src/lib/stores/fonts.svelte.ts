import { invoke } from '@tauri-apps/api/core';

function createFontsStore() {
	let fonts = $state<string[]>([]);
	let loaded = false;

	async function load() {
		if (loaded) return;
		loaded = true;
		try {
			fonts = await invoke<string[]>('list_system_fonts');
		} catch (e) {
			console.error('Failed to load system fonts', e);
		}
	}

	return { get fonts() { return fonts; }, load };
}

export const fontsStore = createFontsStore();
