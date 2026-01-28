import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

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

export async function installAppUpdate(update: Update, onProgress?: (progress: number) => void) {
	let downloaded = 0;
	let contentLength = 0;

	await update.downloadAndInstall((event) => {
		switch (event.event) {
			case 'Started':
				contentLength = event.data.contentLength || 0;
				break;
			case 'Progress':
				downloaded += event.data.chunkLength;
				if (contentLength > 0 && onProgress) {
					onProgress((downloaded / contentLength) * 100);
				}
				break;
			case 'Finished':
				break;
		}
	});

	await relaunch();
}
