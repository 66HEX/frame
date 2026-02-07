import { updateStore } from '$lib/stores/update.svelte';
import { checkForAppUpdate, installAppUpdate } from '$lib/services/update';
import { loadAutoUpdateCheck } from '$lib/services/settings';

export function createAppUpdateManager() {
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

	return {
		initUpdateCheck,
		handleUpdate,
		handleCancelUpdate
	};
}
