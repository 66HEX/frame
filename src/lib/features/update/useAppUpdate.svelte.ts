import { updateStore } from '$lib/stores/update.svelte';
import { openUrl } from '@tauri-apps/plugin-opener';

const GPUI_RELEASE_URL = 'https://github.com/66HEX/frame/releases/tag/0.30.0';
const GPUI_VERSION = '0.30.0';
const MIGRATION_NOTICE = `Frame has moved to a new native GPUI app.

This is the final Tauri bridge release. It cannot install the new app automatically.

To migrate:

1. Uninstall this Tauri version of Frame.
2. Install Frame ${GPUI_VERSION} from GitHub Releases, Homebrew, or WinGet.
3. On Linux, use the managed tarball, AppImage, or Flatpak. GPUI ${GPUI_VERSION} does not ship a DEB package.`;

export function showMigrationNotice() {
	updateStore.isAvailable = true;
	updateStore.version = GPUI_VERSION;
	updateStore.body = MIGRATION_NOTICE;
	updateStore.updateObject = null;
	updateStore.error = null;
	updateStore.isInstalling = false;
	updateStore.progress = 0;
	updateStore.showDialog = true;
}

export function createAppUpdateManager() {
	function initUpdateCheck() {
		showMigrationNotice();
	}

	async function handleUpdate() {
		try {
			updateStore.error = null;
			await openUrl(GPUI_RELEASE_URL);
		} catch (e) {
			console.error('Failed to open GPUI release page:', e);
			updateStore.error = e instanceof Error ? e.message : String(e);
		}
	}

	function handleCancelUpdate() {
		updateStore.showDialog = false;
	}

	return {
		initUpdateCheck,
		showMigrationNotice,
		handleUpdate,
		handleCancelUpdate
	};
}
