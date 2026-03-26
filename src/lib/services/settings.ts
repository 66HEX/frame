import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';

const SETTINGS_STORE_PATH = 'app-settings.dat';
const MAX_CONCURRENCY_KEY = 'maxConcurrency';
const AUTO_UPDATE_CHECK_KEY = 'autoUpdateCheck';

const DEFAULT_MAX_CONCURRENCY = 2;
const DEFAULT_AUTO_UPDATE_CHECK = true;

let storePromise: Promise<Store> | null = null;

async function getStore(): Promise<Store> {
	if (!storePromise) {
		storePromise = Store.load(SETTINGS_STORE_PATH, {
			defaults: {
				[MAX_CONCURRENCY_KEY]: DEFAULT_MAX_CONCURRENCY,
				[AUTO_UPDATE_CHECK_KEY]: DEFAULT_AUTO_UPDATE_CHECK
			}
		});
	}

	return storePromise;
}

export async function loadInitialMaxConcurrency(): Promise<number> {
	try {
		const store = await getStore();
		const stored = await store.get<number>(MAX_CONCURRENCY_KEY);

		if (typeof stored === 'number' && stored > 0) {
			await invoke('set_max_concurrency', { value: stored });
			return stored;
		}
	} catch (error) {
		console.error('Failed to hydrate stored max concurrency', error);
	}

	return invoke<number>('get_max_concurrency');
}

export async function persistMaxConcurrency(value: number): Promise<void> {
	if (!Number.isInteger(value) || value <= 0) {
		throw new Error('Max concurrency must be positive');
	}

	await invoke('set_max_concurrency', { value });
	const store = await getStore();
	await store.set(MAX_CONCURRENCY_KEY, value);
	await store.save();
}

export async function loadAutoUpdateCheck(): Promise<boolean> {
	try {
		const store = await getStore();
		const stored = await store.get<boolean>(AUTO_UPDATE_CHECK_KEY);

		if (typeof stored === 'boolean') {
			return stored;
		}
	} catch (error) {
		console.error('Failed to load auto update check setting', error);
	}

	return DEFAULT_AUTO_UPDATE_CHECK;
}

export async function persistAutoUpdateCheck(value: boolean): Promise<void> {
	const store = await getStore();
	await store.set(AUTO_UPDATE_CHECK_KEY, value);
	await store.save();
}
