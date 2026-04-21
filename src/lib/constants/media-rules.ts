import rawMediaRules from '$lib/shared/media-rules.json';

interface MediaRules {
	allContainers: string[];
	audioOnlyContainers: string[];
	videoOnlyContainers: string[];
	imageContainers?: string[];
	containerVideoCodecCompatibility: Record<string, string[]>;
	containerEncoderPixelFormatCompatibility?: Record<string, Record<string, string[]>>;
	containerVideoStreamCodecCompatibility?: Record<string, string[]>;
	containerAudioCodecCompatibility: Record<string, string[]>;
	containerAudioStreamCodecCompatibility?: Record<string, string[]>;
	containerSubtitleCodecCompatibility: Record<string, string[]>;
	defaultAudioCodec: Record<string, string>;
	defaultAudioCodecFallback: string;
	videoCodecFallbackOrder: string[];
}

const MEDIA_RULES = rawMediaRules as MediaRules;
const ANY_CODEC_TOKEN = '*';

function normalizeContainer(container: string): string {
	return container.toLowerCase();
}

function buildCodecMap(source: Record<string, string[]>): Record<string, Set<string>> {
	return Object.fromEntries(
		Object.entries(source).map(([container, codecs]) => [
			normalizeContainer(container),
			new Set(codecs.map((codec) => codec.toLowerCase()))
		])
	);
}

function buildNestedCodecMap(
	source: Record<string, Record<string, string[]>>
): Record<string, Record<string, Set<string>>> {
	return Object.fromEntries(
		Object.entries(source).map(([container, codecMap]) => [
			normalizeContainer(container),
			Object.fromEntries(
				Object.entries(codecMap).map(([codec, values]) => [
					codec.toLowerCase(),
					new Set(values.map((value) => value.toLowerCase()))
				])
			)
		])
	);
}

const AUDIO_ONLY_CONTAINER_SET = new Set(MEDIA_RULES.audioOnlyContainers.map(normalizeContainer));
const VIDEO_ONLY_CONTAINER_SET = new Set(MEDIA_RULES.videoOnlyContainers.map(normalizeContainer));
const IMAGE_CONTAINER_SET = new Set((MEDIA_RULES.imageContainers ?? []).map(normalizeContainer));
const VIDEO_COMPATIBILITY_MAP = buildCodecMap(MEDIA_RULES.containerVideoCodecCompatibility);
const VIDEO_ENCODER_PIXEL_FORMAT_COMPATIBILITY_MAP = buildNestedCodecMap(
	MEDIA_RULES.containerEncoderPixelFormatCompatibility ?? {}
);
const VIDEO_STREAM_COMPATIBILITY_MAP = buildCodecMap(
	MEDIA_RULES.containerVideoStreamCodecCompatibility ?? {}
);
const AUDIO_COMPATIBILITY_MAP = buildCodecMap(MEDIA_RULES.containerAudioCodecCompatibility);
const AUDIO_STREAM_COMPATIBILITY_MAP = buildCodecMap(
	MEDIA_RULES.containerAudioStreamCodecCompatibility ?? {}
);
const SUBTITLE_COMPATIBILITY_MAP = buildCodecMap(MEDIA_RULES.containerSubtitleCodecCompatibility);
const DEFAULT_AUDIO_CODEC_MAP = Object.fromEntries(
	Object.entries(MEDIA_RULES.defaultAudioCodec).map(([container, codec]) => [
		normalizeContainer(container),
		codec
	])
) as Record<string, string>;

export const ALL_CONTAINERS = Object.freeze([...MEDIA_RULES.allContainers]);
export const AUDIO_ONLY_CONTAINERS = Object.freeze([...MEDIA_RULES.audioOnlyContainers]);
export const IMAGE_CONTAINERS = Object.freeze([...(MEDIA_RULES.imageContainers ?? [])]);
export const VIDEO_CODEC_FALLBACK_ORDER = Object.freeze([...MEDIA_RULES.videoCodecFallbackOrder]);
export const CONTAINER_VIDEO_CODEC_COMPATIBILITY = VIDEO_COMPATIBILITY_MAP;

export function isAudioOnlyContainer(container: string): boolean {
	return AUDIO_ONLY_CONTAINER_SET.has(normalizeContainer(container));
}

export function isVideoOnlyContainer(container: string): boolean {
	return VIDEO_ONLY_CONTAINER_SET.has(normalizeContainer(container));
}

export function isImageContainer(container: string): boolean {
	return IMAGE_CONTAINER_SET.has(normalizeContainer(container));
}

export function containerSupportsAudio(container: string): boolean {
	return !isVideoOnlyContainer(container) && !isImageContainer(container);
}

export function containerSupportsSubtitles(container: string): boolean {
	return (
		!isAudioOnlyContainer(container) &&
		!isVideoOnlyContainer(container) &&
		!isImageContainer(container)
	);
}

export function isGifContainer(container: string): boolean {
	return normalizeContainer(container) === 'gif';
}

export function isVideoCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = VIDEO_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	return allowedCodecs.has(codec.toLowerCase());
}

function getPixelFormatAllowList(
	container: string,
	encoder: string
): Set<string> | null | undefined {
	const containerRules =
		VIDEO_ENCODER_PIXEL_FORMAT_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!containerRules) return undefined;
	return containerRules[encoder.toLowerCase()] ?? containerRules[ANY_CODEC_TOKEN] ?? null;
}

export function isPixelFormatAllowedForContainerAndEncoder(
	container: string,
	encoder: string,
	pixelFormat: string
): boolean {
	const normalizedPixelFormat = pixelFormat.toLowerCase();
	if (normalizedPixelFormat === 'auto') return true;

	const allowedPixelFormats = getPixelFormatAllowList(container, encoder);
	if (allowedPixelFormats === undefined) return true;
	if (allowedPixelFormats === null) return false;
	if (allowedPixelFormats.has(ANY_CODEC_TOKEN)) return true;
	return allowedPixelFormats.has(normalizedPixelFormat);
}

export function getAllowedPixelFormatsForContainerAndEncoder(
	container: string,
	encoder: string,
	candidates: readonly string[]
): string[] {
	const allowedPixelFormats = getPixelFormatAllowList(container, encoder);
	if (allowedPixelFormats === undefined) {
		return [...candidates];
	}
	if (allowedPixelFormats === null) {
		return candidates.filter((format) => format.toLowerCase() === 'auto');
	}
	if (allowedPixelFormats.has(ANY_CODEC_TOKEN)) return [...candidates];

	return candidates.filter((format) => {
		const normalized = format.toLowerCase();
		return normalized === 'auto' || allowedPixelFormats.has(normalized);
	});
}

export function isAudioCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = AUDIO_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	if (allowedCodecs.has(ANY_CODEC_TOKEN)) return true;
	return allowedCodecs.has(codec.toLowerCase());
}

export function isVideoStreamCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = VIDEO_STREAM_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	if (allowedCodecs.has(ANY_CODEC_TOKEN)) return true;
	return allowedCodecs.has(codec.toLowerCase());
}

export function isSubtitleCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = SUBTITLE_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	if (allowedCodecs.has(ANY_CODEC_TOKEN)) return true;
	return allowedCodecs.has(codec.toLowerCase());
}

export function isAudioStreamCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = AUDIO_STREAM_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	if (allowedCodecs.has(ANY_CODEC_TOKEN)) return true;
	return allowedCodecs.has(codec.toLowerCase());
}

export function getDefaultAudioCodecForContainer(container: string): string {
	return (
		DEFAULT_AUDIO_CODEC_MAP[normalizeContainer(container)] ?? MEDIA_RULES.defaultAudioCodecFallback
	);
}
