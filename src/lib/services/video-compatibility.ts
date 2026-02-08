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
	{ id: 'h264_videotoolbox', label: 'H.264 (Apple Silicon)' },
	{ id: 'h264_nvenc', label: 'H.264 (NVIDIA)' },
	{ id: 'hevc_videotoolbox', label: 'H.265 (Apple Silicon)' },
	{ id: 'hevc_nvenc', label: 'H.265 (NVIDIA)' },
	{ id: 'av1_nvenc', label: 'AV1 (NVIDIA)' }
] as const;

export const NVENC_ALLOWED_PRESETS = new Set<VideoPreset>(['fast', 'medium', 'slow']);
export const NVENC_ENCODERS = new Set(['h264_nvenc', 'hevc_nvenc', 'av1_nvenc']);
export const VIDEOTOOLBOX_ENCODERS = new Set(['h264_videotoolbox', 'hevc_videotoolbox']);

export const CONTAINER_VIDEO_CODEC_COMPATIBILITY: Record<string, Set<string>> = {
	mp4: new Set([
		'libx264',
		'libx265',
		'vp9',
		'libsvtav1',
		'h264_videotoolbox',
		'h264_nvenc',
		'hevc_videotoolbox',
		'hevc_nvenc',
		'av1_nvenc'
	]),
	mkv: new Set([
		'libx264',
		'libx265',
		'vp9',
		'prores',
		'libsvtav1',
		'h264_videotoolbox',
		'h264_nvenc',
		'hevc_videotoolbox',
		'hevc_nvenc',
		'av1_nvenc'
	]),
	webm: new Set(['vp9']),
	mov: new Set([
		'libx264',
		'libx265',
		'prores',
		'h264_videotoolbox',
		'h264_nvenc',
		'hevc_videotoolbox',
		'hevc_nvenc'
	])
};

export const VIDEO_CODEC_FALLBACK_ORDER = ['libx264', 'libx265', 'vp9', 'prores', 'libsvtav1'];

export function isVideoPresetAllowed(codec: string, preset: string): boolean {
	if (VIDEOTOOLBOX_ENCODERS.has(codec)) return false;
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
