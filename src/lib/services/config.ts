import { AUDIO_ONLY_CONTAINERS, type ConversionConfig, type SourceMetadata } from '$lib/types';
import { getDefaultAudioCodec, isAudioCodecAllowed } from '$lib/services/media';
import {
	containerSupportsAudio,
	containerSupportsSubtitles,
	isGifContainer
} from '$lib/constants/media-rules';
import {
	NVENC_ENCODERS,
	VIDEOTOOLBOX_ENCODERS,
	getFirstAllowedPreset,
	getFirstAllowedVideoCodec,
	isVideoCodecAllowed,
	isVideoPresetAllowed
} from '$lib/services/video-compatibility';

export function normalizeConversionConfig(
	config: ConversionConfig,
	metadata?: SourceMetadata
): ConversionConfig {
	const next: ConversionConfig = {
		...config,
		selectedAudioTracks: [...(config.selectedAudioTracks ?? [])],
		selectedSubtitleTracks: [...(config.selectedSubtitleTracks ?? [])],
		metadata: { ...config.metadata },
		crop: config.crop ? { ...config.crop } : config.crop
	};

	const isSourceAudioOnly = Boolean(metadata && !metadata.videoCodec);
	if (isSourceAudioOnly && !AUDIO_ONLY_CONTAINERS.includes(next.container)) {
		next.container = 'mp3';
	}

	if (typeof next.gifColors !== 'number' || !Number.isFinite(next.gifColors)) {
		next.gifColors = 256;
	}
	next.gifColors = Math.min(256, Math.max(2, Math.round(next.gifColors)));

	if (typeof next.gifLoop !== 'number' || !Number.isFinite(next.gifLoop)) {
		next.gifLoop = 0;
	}
	next.gifLoop = Math.min(65535, Math.max(0, Math.round(next.gifLoop)));

	const allowedGifDither = new Set(['none', 'bayer', 'floyd_steinberg', 'sierra2_4a']);
	if (!next.gifDither || !allowedGifDither.has(next.gifDither)) {
		next.gifDither = 'sierra2_4a';
	}

	const supportsAudio = containerSupportsAudio(next.container);
	if (supportsAudio && !isAudioCodecAllowed(next.audioCodec, next.container)) {
		next.audioCodec = getDefaultAudioCodec(next.container);
	}

	const isAudioContainer = AUDIO_ONLY_CONTAINERS.includes(next.container);
	const supportsSubtitles = containerSupportsSubtitles(next.container);
	const isGifOutput = isGifContainer(next.container);
	if (isAudioContainer) {
		next.mlUpscale = 'none';
		next.selectedSubtitleTracks = [];
		next.subtitleBurnPath = undefined;
	}

	if (!supportsAudio) {
		next.selectedAudioTracks = [];
		next.audioNormalize = false;
	}

	if (!supportsSubtitles) {
		next.selectedSubtitleTracks = [];
		next.subtitleBurnPath = undefined;
	}

	if (isGifOutput) {
		next.videoCodec = 'gif';
		next.videoBitrateMode = 'crf';
		next.mlUpscale = 'none';
		next.hwDecode = false;
		next.nvencSpatialAq = false;
		next.nvencTemporalAq = false;
		next.videotoolboxAllowSw = false;
	}

	if (!isAudioContainer && !isVideoCodecAllowed(next.container, next.videoCodec)) {
		next.videoCodec = getFirstAllowedVideoCodec(next.container);
	}

	if (next.mlUpscale && next.mlUpscale !== 'none' && next.resolution !== 'original') {
		next.resolution = 'original';
	}

	if (!isVideoPresetAllowed(next.videoCodec, next.preset)) {
		next.preset = getFirstAllowedPreset(next.videoCodec);
	}

	if (!NVENC_ENCODERS.has(next.videoCodec)) {
		next.nvencSpatialAq = false;
		next.nvencTemporalAq = false;
	}

	if (!VIDEOTOOLBOX_ENCODERS.has(next.videoCodec)) {
		next.videotoolboxAllowSw = false;
	}

	if (!NVENC_ENCODERS.has(next.videoCodec) && !VIDEOTOOLBOX_ENCODERS.has(next.videoCodec)) {
		next.hwDecode = false;
	}

	return next;
}
