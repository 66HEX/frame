import { AUDIO_ONLY_CONTAINERS, type ConversionConfig, type SourceMetadata } from '$lib/types';
import { getDefaultAudioCodec, isAudioCodecAllowed } from '$lib/services/media';
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

	if (!isAudioCodecAllowed(next.audioCodec, next.container)) {
		next.audioCodec = getDefaultAudioCodec(next.container);
	}

	const isAudioContainer = AUDIO_ONLY_CONTAINERS.includes(next.container);
	if (isAudioContainer) {
		next.mlUpscale = 'none';
		next.selectedSubtitleTracks = [];
		next.subtitleBurnPath = undefined;
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
