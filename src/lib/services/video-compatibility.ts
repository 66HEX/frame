import {
	CONTAINER_VIDEO_CODEC_COMPATIBILITY as SHARED_CONTAINER_VIDEO_CODEC_COMPATIBILITY,
	getAllowedPixelFormatsForContainerAndEncoder,
	isPixelFormatAllowedForContainerAndEncoder,
	VIDEO_CODEC_FALLBACK_ORDER as SHARED_VIDEO_CODEC_FALLBACK_ORDER
} from '$lib/constants/media-rules';

export const VIDEO_PRESETS = [
	'ultrafast',
	'superfast',
	'veryfast',
	'faster',
	'fast',
	'medium',
	'slow',
	'slower',
	'veryslow'
] as const;

type VideoPreset = (typeof VIDEO_PRESETS)[number];

export const VIDEO_CODEC_OPTIONS = [
	{ id: 'libx264', label: 'H.264 / AVC' },
	{ id: 'libx265', label: 'H.265 / HEVC' },
	{ id: 'vp9', label: 'VP9 / Web' },
	{ id: 'prores', label: 'Apple ProRes' },
	{ id: 'libsvtav1', label: 'AV1 / SVT' },
	{ id: 'gif', label: 'GIF / Palette' },
	{ id: 'h264_videotoolbox', label: 'H.264 (Apple Silicon)' },
	{ id: 'h264_nvenc', label: 'H.264 (NVIDIA)' },
	{ id: 'hevc_videotoolbox', label: 'H.265 (Apple Silicon)' },
	{ id: 'hevc_nvenc', label: 'H.265 (NVIDIA)' },
	{ id: 'av1_nvenc', label: 'AV1 (NVIDIA)' }
] as const;

export const VIDEO_PIXEL_FORMAT_OPTIONS = [
	{ id: 'auto', label: 'Auto' },
	{ id: 'yuv420p', label: 'YUV 4:2:0 (8-bit)' },
	{ id: 'yuv422p', label: 'YUV 4:2:2 (8-bit)' },
	{ id: 'yuv444p', label: 'YUV 4:4:4 (8-bit)' },
	{ id: 'yuv420p10le', label: 'YUV 4:2:0 (10-bit)' },
	{ id: 'yuv422p10le', label: 'YUV 4:2:2 (10-bit)' },
	{ id: 'yuv444p10le', label: 'YUV 4:4:4 (10-bit)' }
] as const;
export type VideoPixelFormatId = (typeof VIDEO_PIXEL_FORMAT_OPTIONS)[number]['id'];

export const NVENC_ALLOWED_PRESETS = new Set<VideoPreset>(['fast', 'medium', 'slow']);
export const NVENC_ENCODERS = new Set(['h264_nvenc', 'hevc_nvenc', 'av1_nvenc']);
export const VIDEOTOOLBOX_ENCODERS = new Set(['h264_videotoolbox', 'hevc_videotoolbox']);

export const CONTAINER_VIDEO_CODEC_COMPATIBILITY = SHARED_CONTAINER_VIDEO_CODEC_COMPATIBILITY;

export const VIDEO_CODEC_FALLBACK_ORDER = SHARED_VIDEO_CODEC_FALLBACK_ORDER;

export function isVideoPresetAllowed(codec: string, preset: string): boolean {
	if (VIDEOTOOLBOX_ENCODERS.has(codec)) return true;
	if (NVENC_ENCODERS.has(codec)) return NVENC_ALLOWED_PRESETS.has(preset as VideoPreset);
	return VIDEO_PRESETS.includes(preset as VideoPreset);
}

export function getFirstAllowedPreset(codec: string): string {
	return VIDEO_PRESETS.find((preset) => isVideoPresetAllowed(codec, preset)) ?? 'medium';
}

export function isVideoCodecAllowed(container: string, codec: string): boolean {
	const allowed = CONTAINER_VIDEO_CODEC_COMPATIBILITY[container];
	if (!allowed) return true;
	return allowed.has(codec);
}

export function getFirstAllowedVideoCodec(
	container: string,
	candidates: readonly string[] = VIDEO_CODEC_FALLBACK_ORDER
): string {
	const allowed = CONTAINER_VIDEO_CODEC_COMPATIBILITY[container];
	if (!allowed || allowed.size === 0) return candidates[0] ?? VIDEO_CODEC_FALLBACK_ORDER[0];

	for (const codec of candidates) {
		if (allowed.has(codec)) return codec;
	}

	return allowed.values().next().value ?? candidates[0] ?? VIDEO_CODEC_FALLBACK_ORDER[0];
}

export function isVideoPixelFormatAllowed(
	container: string,
	encoder: string,
	pixelFormat: string
): boolean {
	return isPixelFormatAllowedForContainerAndEncoder(container, encoder, pixelFormat);
}

export function getAllowedVideoPixelFormats(
	container: string,
	encoder: string
): VideoPixelFormatId[] {
	return getAllowedPixelFormatsForContainerAndEncoder(
		container,
		encoder,
		VIDEO_PIXEL_FORMAT_OPTIONS.map((option) => option.id)
	) as VideoPixelFormatId[];
}

export function getFirstAllowedVideoPixelFormat(
	container: string,
	encoder: string
): VideoPixelFormatId {
	return getAllowedVideoPixelFormats(container, encoder)[0] ?? 'auto';
}
