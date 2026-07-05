import { check, type Update } from '@tauri-apps/plugin-updater';

export type UpdateCheckResult = {
	available: boolean;
	version?: string;
	body?: string;
	date?: string;
	updateObject: Update | null;
};

export async function checkForAppUpdate(): Promise<UpdateCheckResult> {
	try {
		const update = await check();
		if (update) {
			return {
				available: true,
				version: update.version,
				body: update.body,
				date: update.date,
				updateObject: update
			};
		}
		return { available: false, updateObject: null };
	} catch (error) {
		console.error('Failed to check for updates:', error);
		throw error;
	}
}
