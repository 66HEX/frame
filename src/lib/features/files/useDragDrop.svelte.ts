import { listen } from '@tauri-apps/api/event';

export interface DragDropCallbacks {
	onFilesDropped: (paths: string[]) => void;
}

export function createDragDropManager(callbacks: DragDropCallbacks) {
	let isDragging = $state(false);

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
				callbacks.onFilesDropped(event.payload.paths);
			}
		});

		return () => {
			unlistenEnter();
			unlistenLeave();
			unlistenDrop();
		};
	}

	return {
		get isDragging() {
			return isDragging;
		},
		setupDragDrop
	};
}
