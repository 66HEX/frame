import type { ConversionConfig, CropSettings } from '$lib/types';
import {
	type CropRect,
	type DragHandle,
	getAspectValue,
	clampRect,
	transformCropRect,
	adjustRectToRatio,
	applyVisualCropDrag
} from '$lib/utils/crop';

interface PreviewCropOptions {
	getRotation: () => ConversionConfig['rotation'];
	getFlipHorizontal: () => boolean;
	getFlipVertical: () => boolean;
	getSourceWidth: () => number | undefined;
	getSourceHeight: () => number | undefined;
	getControlsDisabled: () => boolean;
	onUpdateConfig?: (config: Partial<ConversionConfig>) => void;
}

export function createPreviewCrop({
	getRotation,
	getFlipHorizontal,
	getFlipVertical,
	getSourceWidth,
	getSourceHeight,
	getControlsDisabled,
	onUpdateConfig
}: PreviewCropOptions) {
	let naturalWidth = $state(0);
	let naturalHeight = $state(0);

	let cropMode = $state(false);
	let appliedCrop = $state<CropRect | null>(null);
	let draftCrop = $state<CropRect | null>(null);
	let cropAspect = $state<string>('free');
	let cropHandle = $state<DragHandle | null>(null);
	let cropDragOrigin = $state<{
		handle: DragHandle;
		startRect: CropRect;
		startX: number;
		startY: number;
	} | null>(null);

	const isSideRotation = $derived(getRotation() === '90' || getRotation() === '270');

	const hasCropDimensions = $derived.by(() => {
		const baseWidth = getSourceWidth() ?? naturalWidth;
		const baseHeight = getSourceHeight() ?? naturalHeight;
		if (!baseWidth || !baseHeight) return false;
		if (getRotation() === '90' || getRotation() === '270') {
			return Boolean(baseHeight && baseWidth);
		}
		return true;
	});

	function setNaturalDimensions(width: number, height: number) {
		naturalWidth = width;
		naturalHeight = height;
	}

	function syncInitialCrop(initialCrop?: CropSettings | null) {
		if (initialCrop?.enabled && initialCrop.sourceWidth && initialCrop.sourceHeight) {
			const rawRect = {
				x: initialCrop.x / initialCrop.sourceWidth,
				y: initialCrop.y / initialCrop.sourceHeight,
				width: initialCrop.width / initialCrop.sourceWidth,
				height: initialCrop.height / initialCrop.sourceHeight
			};

			appliedCrop = clampRect(
				transformCropRect(rawRect, getRotation(), getFlipHorizontal(), getFlipVertical(), true)
			);

			cropAspect = initialCrop.aspectRatio ?? 'free';
		} else if (!cropMode) {
			appliedCrop = null;
			cropAspect = 'free';
		}

		if (!cropMode) {
			draftCrop = null;
		}
	}

	function defaultCropRect(): CropRect {
		return { x: 0.1, y: 0.1, width: 0.8, height: 0.8 };
	}

	function getBaseDimensions(rot: ConversionConfig['rotation'] = getRotation()) {
		let baseWidth = getSourceWidth() ?? naturalWidth;
		let baseHeight = getSourceHeight() ?? naturalHeight;
		if (!baseWidth || !baseHeight) return null;
		if (rot === '90' || rot === '270') {
			[baseWidth, baseHeight] = [baseHeight, baseWidth];
		}
		return { width: baseWidth, height: baseHeight };
	}

	function persistCrop(rect: CropRect | null, overrides: Partial<ConversionConfig> = {}) {
		if (!onUpdateConfig) return;

		const nextRotation = overrides.rotation ?? getRotation();
		const nextFlipH = overrides.flipHorizontal ?? getFlipHorizontal();
		const nextFlipV = overrides.flipVertical ?? getFlipVertical();

		if (!rect) {
			onUpdateConfig({ crop: null, ...overrides });
			return;
		}

		const dims = getBaseDimensions(nextRotation);
		if (!dims) return;

		const outputRect = transformCropRect(rect, nextRotation, nextFlipH, nextFlipV, false);

		const payload: CropSettings = {
			enabled: true,
			x: Math.round(outputRect.x * dims.width),
			y: Math.round(outputRect.y * dims.height),
			width: Math.round(outputRect.width * dims.width),
			height: Math.round(outputRect.height * dims.height),
			sourceWidth: dims.width,
			sourceHeight: dims.height,
			aspectRatio: cropAspect === 'free' ? null : cropAspect
		};

		onUpdateConfig({ crop: payload, ...overrides });
	}

	function toggleCropMode() {
		if (getControlsDisabled() || !hasCropDimensions) return;

		if (cropMode) {
			cropMode = false;
			draftCrop = null;
			return;
		}

		cropMode = true;
		draftCrop = appliedCrop ? { ...appliedCrop } : defaultCropRect();
	}

	function selectAspect(id: string) {
		cropAspect = id;
		if (!draftCrop) return;
		if (id === 'free') {
			draftCrop = clampRect({ ...draftCrop });
			return;
		}

		const ratio = getAspectValue(id);
		if (!ratio) return;
		const width = getSourceWidth() ?? naturalWidth;
		const height = getSourceHeight() ?? naturalHeight;
		draftCrop = clampRect(adjustRectToRatio(draftCrop, ratio, width, height, isSideRotation));
	}

	function applyCrop() {
		if (!draftCrop || !hasCropDimensions) return;

		const isFull =
			draftCrop.x <= 0.001 &&
			draftCrop.y <= 0.001 &&
			draftCrop.width >= 0.999 &&
			draftCrop.height >= 0.999;

		if (isFull) {
			persistCrop(null);
			appliedCrop = null;
			cropAspect = 'free';
		} else {
			persistCrop(draftCrop);
			appliedCrop = { ...draftCrop };
		}

		cropMode = false;
		draftCrop = null;
	}

	function resetCropSelection() {
		if (!draftCrop) {
			draftCrop = defaultCropRect();
		} else {
			draftCrop = { x: 0, y: 0, width: 1, height: 1 };
		}

		cropAspect = 'free';
	}

	function handleRotateToggle() {
		if (!onUpdateConfig || getControlsDisabled()) return;
		const rotation = getRotation();
		const steps: ConversionConfig['rotation'][] = ['0', '90', '180', '270'];
		const index = steps.indexOf(rotation);
		const next = steps[(index + 1) % steps.length];

		if (appliedCrop) {
			persistCrop(appliedCrop, { rotation: next });
		} else {
			onUpdateConfig({ rotation: next });
		}
	}

	function toggleFlip(axis: 'horizontal' | 'vertical') {
		if (!onUpdateConfig || getControlsDisabled()) return;

		const nextFlipH = axis === 'horizontal' ? !getFlipHorizontal() : getFlipHorizontal();
		const nextFlipV = axis === 'vertical' ? !getFlipVertical() : getFlipVertical();

		if (appliedCrop) {
			persistCrop(appliedCrop, { flipHorizontal: nextFlipH, flipVertical: nextFlipV });
		} else {
			onUpdateConfig({ flipHorizontal: nextFlipH, flipVertical: nextFlipV });
		}
	}

	function beginCropDrag(handle: DragHandle, point: { x: number; y: number }) {
		if (!draftCrop || !cropMode) return;
		cropHandle = handle;
		cropDragOrigin = {
			handle,
			startRect: { ...draftCrop },
			startX: point.x,
			startY: point.y
		};
	}

	function updateCropDrag(point: { x: number; y: number }) {
		if (!cropHandle || !cropDragOrigin || !draftCrop) {
			return;
		}

		const startRect = cropDragOrigin.startRect;
		const width = getSourceWidth() ?? naturalWidth;
		const height = getSourceHeight() ?? naturalHeight;

		draftCrop = applyVisualCropDrag({
			startRect,
			handle: cropHandle,
			startPoint: { x: cropDragOrigin.startX, y: cropDragOrigin.startY },
			currentPoint: point,
			aspectId: cropAspect,
			sourceWidth: width,
			sourceHeight: height,
			isSideRotation
		});
	}

	function endCropDrag() {
		cropHandle = null;
		cropDragOrigin = null;
	}

	function destroy() {
		endCropDrag();
	}

	return {
		get cropMode() {
			return cropMode;
		},
		get appliedCrop() {
			return appliedCrop;
		},
		get draftCrop() {
			return draftCrop;
		},
		get cropAspect() {
			return cropAspect;
		},
		get hasCropDimensions() {
			return hasCropDimensions;
		},
		get isSideRotation() {
			return isSideRotation;
		},
		setNaturalDimensions,
		syncInitialCrop,
		toggleCropMode,
		selectAspect,
		applyCrop,
		resetCropSelection,
		handleRotateToggle,
		toggleFlip,
		beginCropDrag,
		updateCropDrag,
		endCropDrag,
		destroy
	};
}

export type PreviewCropController = ReturnType<typeof createPreviewCrop>;
