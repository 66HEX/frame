import rawMediaRules from '$lib/shared/media-rules.json';

interface MediaRules {
	allContainers: string[];
	audioOnlyContainers: string[];
	videoOnlyContainers: string[];
	containerVideoCodecCompatibility: Record<string, string[]>;
	containerAudioCodecCompatibility: Record<string, string[]>;
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
			new Set(codecs)
		])
	);
}

const AUDIO_ONLY_CONTAINER_SET = new Set(MEDIA_RULES.audioOnlyContainers.map(normalizeContainer));
const VIDEO_ONLY_CONTAINER_SET = new Set(MEDIA_RULES.videoOnlyContainers.map(normalizeContainer));
const VIDEO_COMPATIBILITY_MAP = buildCodecMap(MEDIA_RULES.containerVideoCodecCompatibility);
const AUDIO_COMPATIBILITY_MAP = buildCodecMap(MEDIA_RULES.containerAudioCodecCompatibility);
const DEFAULT_AUDIO_CODEC_MAP = Object.fromEntries(
	Object.entries(MEDIA_RULES.defaultAudioCodec).map(([container, codec]) => [
		normalizeContainer(container),
		codec
	])
) as Record<string, string>;

export const ALL_CONTAINERS = Object.freeze([...MEDIA_RULES.allContainers]);
export const AUDIO_ONLY_CONTAINERS = Object.freeze([...MEDIA_RULES.audioOnlyContainers]);
export const VIDEO_CODEC_FALLBACK_ORDER = Object.freeze([...MEDIA_RULES.videoCodecFallbackOrder]);
export const CONTAINER_VIDEO_CODEC_COMPATIBILITY = VIDEO_COMPATIBILITY_MAP;

export function isAudioOnlyContainer(container: string): boolean {
	return AUDIO_ONLY_CONTAINER_SET.has(normalizeContainer(container));
}

export function isVideoOnlyContainer(container: string): boolean {
	return VIDEO_ONLY_CONTAINER_SET.has(normalizeContainer(container));
}

export function containerSupportsAudio(container: string): boolean {
	return !isVideoOnlyContainer(container);
}

export function containerSupportsSubtitles(container: string): boolean {
	return !isAudioOnlyContainer(container) && !isVideoOnlyContainer(container);
}

export function isGifContainer(container: string): boolean {
	return normalizeContainer(container) === 'gif';
}

export function isVideoCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = VIDEO_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	return allowedCodecs.has(codec);
}

export function isAudioCodecAllowedForContainer(container: string, codec: string): boolean {
	const allowedCodecs = AUDIO_COMPATIBILITY_MAP[normalizeContainer(container)];
	if (!allowedCodecs) return true;
	if (allowedCodecs.has(ANY_CODEC_TOKEN)) return true;
	return allowedCodecs.has(codec);
}

export function getDefaultAudioCodecForContainer(container: string): string {
	return (
		DEFAULT_AUDIO_CODEC_MAP[normalizeContainer(container)] ?? MEDIA_RULES.defaultAudioCodecFallback
	);
}
