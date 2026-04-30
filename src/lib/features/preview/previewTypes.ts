import type { CropRect } from '$lib/utils/crop';
import type { OverlaySettings } from '$lib/types';

export type PreviewMediaKind = 'unknown' | 'video' | 'audio' | 'image';

export interface PreviewPresentationState {
	mediaKind: PreviewMediaKind;
	rotation: '0' | '90' | '180' | '270';
	flipHorizontal: boolean;
	flipVertical: boolean;
	cropMode: boolean;
	appliedCrop: CropRect | null;
	draftCrop: CropRect | null;
	overlayMode: boolean;
	overlay: OverlaySettings | null;
	sourceWidth?: number;
	sourceHeight?: number;
}

export interface PreviewSource {
	filePath: string;
	mediaKind: PreviewMediaKind;
}

export interface PreviewTransform {
	zoom: number;
	offsetX: number;
	offsetY: number;
}
