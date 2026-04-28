import type { PreviewPresentationState, PreviewTransform } from './previewTypes';

export const MAX_PREVIEW_DPR = 2;
export const DEFAULT_PREVIEW_ZOOM = 1;
export const MIN_PREVIEW_ZOOM = 0.25;
export const MAX_PREVIEW_ZOOM = 8;
export const PREVIEW_ZOOM_STEP = 1.05;
export const PREVIEW_PAN_OVERSCROLL = 2;
export const PREVIEW_TRANSFORM_LERP = 0.2;
export const PREVIEW_TRANSFORM_EPSILON = 0.01;

export interface PreviewSceneState {
	presentation: PreviewPresentationState;
	naturalWidth: number;
	naturalHeight: number;
	wrapperWidth: number;
	wrapperHeight: number;
}

export function getPreviewResolution(): number {
	if (typeof window === 'undefined') return 1;
	return Math.max(1, Math.min(window.devicePixelRatio || 1, MAX_PREVIEW_DPR));
}

export function clampValue(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, value));
}

export function defaultPreviewTransform(): PreviewTransform {
	return { zoom: DEFAULT_PREVIEW_ZOOM, offsetX: 0, offsetY: 0 };
}

export function getSceneMetrics(state: PreviewSceneState, zoom: number) {
	const { presentation, naturalWidth, naturalHeight, wrapperWidth, wrapperHeight } = state;
	const baseWidth = presentation.sourceWidth ?? naturalWidth;
	const baseHeight = presentation.sourceHeight ?? naturalHeight;
	if (!baseWidth || !baseHeight || !wrapperWidth || !wrapperHeight) return null;

	const cropRect =
		!presentation.cropMode && presentation.appliedCrop
			? presentation.appliedCrop
			: { x: 0, y: 0, width: 1, height: 1 };

	const contentWidth = baseWidth * cropRect.width;
	const contentHeight = baseHeight * cropRect.height;
	const sideRotation = presentation.rotation === '90' || presentation.rotation === '270';
	const visualWidth = sideRotation ? contentHeight : contentWidth;
	const visualHeight = sideRotation ? contentWidth : contentHeight;
	const fitScale = Math.min(wrapperWidth / visualWidth, wrapperHeight / visualHeight);
	const cropCenterX = cropRect.x + cropRect.width / 2 - 0.5;
	const cropCenterY = cropRect.y + cropRect.height / 2 - 0.5;

	return {
		baseWidth,
		baseHeight,
		cropCenterX,
		cropCenterY,
		fitScale,
		displayedWidth: visualWidth * fitScale * zoom,
		displayedHeight: visualHeight * fitScale * zoom
	};
}

export function clampPreviewTransform(
	state: PreviewSceneState,
	transform: PreviewTransform
): PreviewTransform {
	const zoom = clampValue(transform.zoom, MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
	const metrics = getSceneMetrics(state, zoom);

	if (!metrics) {
		return defaultPreviewTransform();
	}

	const maxOffsetX =
		Math.max(state.wrapperWidth * PREVIEW_PAN_OVERSCROLL, metrics.displayedWidth) / 2;
	const maxOffsetY =
		Math.max(state.wrapperHeight * PREVIEW_PAN_OVERSCROLL, metrics.displayedHeight) / 2;

	return {
		zoom,
		offsetX: clampValue(transform.offsetX, -maxOffsetX, maxOffsetX),
		offsetY: clampValue(transform.offsetY, -maxOffsetY, maxOffsetY)
	};
}
