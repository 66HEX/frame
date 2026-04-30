import type { ConversionConfig, OverlaySettings } from '$lib/types';

const DEFAULT_OVERLAY_WIDTH = 0.18;
export const MIN_OVERLAY_WIDTH = 0.03;
export const MAX_OVERLAY_WIDTH = 0.8;

type OverlayDragHandle = 'move' | 'nw' | 'ne' | 'se' | 'sw';

interface PreviewOverlayOptions {
	getControlsDisabled: () => boolean;
	onUpdateConfig?: (config: Partial<ConversionConfig>) => void;
	onDeactivateCrop?: () => void;
}

export interface OverlayDragPoint {
	x: number;
	y: number;
	width?: number;
	height?: number;
}

function clamp(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, value));
}

function clampOverlayCenter(
	x: number,
	y: number,
	width: number,
	height = width
): { x: number; y: number } {
	const halfWidth = Math.min(width / 2, 0.5);
	const halfHeight = Math.min(height / 2, 0.5);
	return {
		x: clamp(x, halfWidth, 1 - halfWidth),
		y: clamp(y, halfHeight, 1 - halfHeight)
	};
}

function getMaxOverlayWidth(heightRatio = 1): number {
	if (!Number.isFinite(heightRatio) || heightRatio <= 0) return MAX_OVERLAY_WIDTH;
	return Math.min(MAX_OVERLAY_WIDTH, 1 / heightRatio);
}

function clampOverlayWidth(width: number, heightRatio = 1): number {
	const maxWidth = getMaxOverlayWidth(heightRatio);
	const minWidth = Math.min(MIN_OVERLAY_WIDTH, maxWidth);
	return clamp(width, minWidth, maxWidth);
}

function createDefaultOverlay(path: string): OverlaySettings {
	const width = DEFAULT_OVERLAY_WIDTH;
	const center = clampOverlayCenter(0.5, 0.5, width);
	return {
		enabled: true,
		path,
		x: center.x,
		y: center.y,
		width,
		opacity: 1,
		anchor: 'custom'
	};
}

function normalizeOverlay(overlay: OverlaySettings): OverlaySettings {
	const width = clamp(overlay.width, MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
	const center = clampOverlayCenter(overlay.x, overlay.y, width);
	return {
		...overlay,
		enabled: overlay.enabled,
		path: overlay.path,
		x: center.x,
		y: center.y,
		width,
		opacity: clamp(overlay.opacity, 0, 1),
		anchor: 'custom'
	};
}

export function createPreviewOverlay({
	getControlsDisabled,
	onUpdateConfig,
	onDeactivateCrop
}: PreviewOverlayOptions) {
	let overlayMode = $state(false);
	let overlay = $state<OverlaySettings | null>(null);
	let dragOrigin = $state<{
		handle: OverlayDragHandle;
		startOverlay: OverlaySettings;
		startPoint: OverlayDragPoint;
	} | null>(null);

	function persist(nextOverlay: OverlaySettings | null) {
		overlay = nextOverlay;
		onUpdateConfig?.({ overlay: nextOverlay });
	}

	function syncInitialOverlay(initialOverlay?: OverlaySettings | null) {
		if (dragOrigin) return;
		overlay =
			initialOverlay?.enabled && initialOverlay.path ? normalizeOverlay(initialOverlay) : null;
		if (!overlay) {
			overlayMode = false;
		}
	}

	function setOverlayFromPath(path: string) {
		if (getControlsDisabled()) return;
		onDeactivateCrop?.();
		const nextOverlay = createDefaultOverlay(path);
		overlayMode = true;
		persist(nextOverlay);
	}

	function toggleOverlayMode() {
		if (getControlsDisabled() || !overlay) return;
		if (!overlayMode) {
			onDeactivateCrop?.();
		}
		overlayMode = !overlayMode;
	}

	function setOverlayMode(value: boolean) {
		if (getControlsDisabled() && value) return;
		if (value) {
			onDeactivateCrop?.();
		}
		overlayMode = value && Boolean(overlay);
	}

	function beginOverlayDrag(handle: OverlayDragHandle, point: OverlayDragPoint) {
		if (!overlay || !overlayMode || getControlsDisabled()) return;
		dragOrigin = {
			handle,
			startOverlay: { ...overlay, width: point.width ?? overlay.width },
			startPoint: point
		};
	}

	function updateOverlayDrag(point: OverlayDragPoint) {
		if (!overlay || !dragOrigin) return;
		const { handle, startOverlay, startPoint } = dragOrigin;

		if (handle === 'move') {
			const nextCenter = clampOverlayCenter(
				startOverlay.x + point.x - startPoint.x,
				startOverlay.y + point.y - startPoint.y,
				startOverlay.width,
				startPoint.height ?? startOverlay.width
			);
			persist({
				...startOverlay,
				x: nextCenter.x,
				y: nextCenter.y,
				anchor: 'custom'
			});
			return;
		}

		if (!point.width || !startPoint.width || !startPoint.height) return;
		const startLeft = startOverlay.x - startOverlay.width / 2;
		const startRight = startOverlay.x + startOverlay.width / 2;
		const startTop = startOverlay.y - startPoint.height / 2;
		const startBottom = startOverlay.y + startPoint.height / 2;
		const anchorX = handle === 'nw' || handle === 'sw' ? startRight : startLeft;
		const anchorY = handle === 'nw' || handle === 'ne' ? startBottom : startTop;
		const aspect = startPoint.height / startPoint.width;
		const rawWidthFromX = Math.abs(point.x - anchorX);
		const rawWidthFromY = Math.abs(point.y - anchorY) / aspect;
		const width = clampOverlayWidth(Math.max(rawWidthFromX, rawWidthFromY), aspect);
		const height = width * aspect;
		const directionX = handle === 'nw' || handle === 'sw' ? -1 : 1;
		const directionY = handle === 'nw' || handle === 'ne' ? -1 : 1;
		const nextCenter = clampOverlayCenter(
			anchorX + (directionX * width) / 2,
			anchorY + (directionY * height) / 2,
			width,
			height
		);

		persist({
			...startOverlay,
			x: nextCenter.x,
			y: nextCenter.y,
			width,
			anchor: 'custom'
		});
	}

	function endOverlayDrag() {
		dragOrigin = null;
	}

	function setOpacity(value: number) {
		if (!overlay || getControlsDisabled()) return;
		persist({ ...overlay, opacity: clamp(value, 0, 1) });
	}

	function nudgeSize(direction: 1 | -1, heightRatio?: number) {
		if (!overlay || getControlsDisabled()) return;
		const width = clampOverlayWidth(overlay.width + direction * 0.025, heightRatio);
		const center = clampOverlayCenter(overlay.x, overlay.y, width, width * (heightRatio ?? 1));
		persist({
			...overlay,
			x: center.x,
			y: center.y,
			width,
			anchor: 'custom'
		});
	}

	function removeOverlay() {
		if (getControlsDisabled()) return;
		overlayMode = false;
		persist(null);
	}

	function destroy() {
		endOverlayDrag();
	}

	return {
		get overlayMode() {
			return overlayMode;
		},
		get overlay() {
			return overlay;
		},
		get isDragging() {
			return Boolean(dragOrigin);
		},
		syncInitialOverlay,
		setOverlayFromPath,
		toggleOverlayMode,
		setOverlayMode,
		beginOverlayDrag,
		updateOverlayDrag,
		endOverlayDrag,
		setOpacity,
		nudgeSize,
		removeOverlay,
		destroy
	};
}

export type PreviewOverlayController = ReturnType<typeof createPreviewOverlay>;
