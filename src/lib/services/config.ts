import { AUDIO_ONLY_CONTAINERS, type ConversionConfig, type SourceMetadata } from '$lib/types';
import {
	audioCodecSupportsVbr,
	getAudioQualityRange,
	getDefaultAudioCodec,
	isAudioCodecAllowed
} from '$lib/services/media';
import { capabilities } from '$lib/stores/capabilities.svelte';
import {
	containerSupportsAudio,
	containerSupportsSubtitles,
	isGifContainer,
	isImageContainer
} from '$lib/constants/media-rules';
import {
	NVENC_ENCODERS,
	VIDEO_PIXEL_FORMAT_OPTIONS,
	VIDEOTOOLBOX_ENCODERS,
	getFirstAllowedVideoPixelFormat,
	getFirstAllowedPreset,
	getFirstAllowedVideoCodec,
	isVideoPixelFormatAllowed,
	isVideoCodecAllowed,
	isVideoPresetAllowed
} from '$lib/services/video-compatibility';

export function normalizeConversionConfig(
	config: ConversionConfig,
	metadata?: SourceMetadata
): ConversionConfig {
	const next: ConversionConfig = {
		...config,
		processingMode: config.processingMode === 'copy' ? 'copy' : 'reencode',
		selectedAudioTracks: [...(config.selectedAudioTracks ?? [])],
		selectedSubtitleTracks: [...(config.selectedSubtitleTracks ?? [])],
		metadata: { ...config.metadata },
		crop: config.crop ? { ...config.crop } : config.crop
	};

	const requestedPixelFormat =
		typeof next.pixelFormat === 'string' ? next.pixelFormat.trim() : 'auto';
	next.pixelFormat = VIDEO_PIXEL_FORMAT_OPTIONS.map((option) => option.id).includes(
		requestedPixelFormat as (typeof VIDEO_PIXEL_FORMAT_OPTIONS)[number]['id']
	)
		? (requestedPixelFormat as ConversionConfig['pixelFormat'])
		: 'auto';

	const sourceKind = metadata?.mediaKind ?? (metadata && !metadata.videoCodec ? 'audio' : 'video');
	const isSourceAudioOnly = sourceKind === 'audio';
	const isSourceImage = sourceKind === 'image';
	if (isSourceAudioOnly && !AUDIO_ONLY_CONTAINERS.includes(next.container)) {
		next.container = 'mp3';
	}
	if (isSourceImage && !isImageContainer(next.container) && next.container !== 'gif') {
		next.container = 'png';
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

	// If libfdk_aac isn't present in this FFmpeg build, fall back to native aac.
	if (next.audioCodec === 'libfdk_aac' && !capabilities.encoders.libfdk_aac) {
		next.audioCodec = 'aac';
	}

	// Audio bitrate mode: force CBR when the codec doesn't support VBR, and
	// clamp the quality value into the codec-specific range. This mirrors how
	// video preset / pixel-format normalisation works above.
	if (next.audioBitrateMode !== 'vbr' && next.audioBitrateMode !== 'bitrate') {
		next.audioBitrateMode = 'bitrate';
	}
	if (next.audioBitrateMode === 'vbr' && !audioCodecSupportsVbr(next.audioCodec)) {
		next.audioBitrateMode = 'bitrate';
	}
	const qualityRange = getAudioQualityRange(next.audioCodec);
	if (qualityRange) {
		const parsed = Number.parseInt(next.audioQuality ?? '', 10);
		const resolved = Number.isFinite(parsed)
			? Math.min(qualityRange.max, Math.max(qualityRange.min, parsed))
			: qualityRange.defaultValue;
		next.audioQuality = String(resolved);
	} else if (!next.audioQuality) {
		next.audioQuality = '4';
	}

	const isAudioContainer = AUDIO_ONLY_CONTAINERS.includes(next.container);
	const supportsSubtitles = containerSupportsSubtitles(next.container);
	const isGifOutput = isGifContainer(next.container);

	if (isGifOutput && next.processingMode === 'copy') {
		next.processingMode = 'reencode';
	}

	const isCopyMode = next.processingMode === 'copy';

	if (isCopyMode) {
		next.pixelFormat = 'auto';
		next.subtitleBurnPath = undefined;
		next.audioNormalize = false;
		next.audioVolume = 100;
		next.audioBitrateMode = 'bitrate';
		next.resolution = 'original';
		next.customWidth = undefined;
		next.customHeight = undefined;
		next.fps = 'original';
		next.rotation = '0';
		next.flipHorizontal = false;
		next.flipVertical = false;
		next.crop = null;
		next.mlUpscale = 'none';
		next.hwDecode = false;
		next.nvencSpatialAq = false;
		next.nvencTemporalAq = false;
		next.videotoolboxAllowSw = false;
	}

	if (isSourceImage) {
		next.processingMode = 'reencode';
		next.startTime = undefined;
		next.endTime = undefined;
		next.selectedAudioTracks = [];
		next.selectedSubtitleTracks = [];
		next.subtitleBurnPath = undefined;
		next.audioNormalize = false;
		next.audioVolume = 100;
		next.metadata = {
			...next.metadata,
			album: undefined,
			genre: undefined
		};
	}

	if (isAudioContainer) {
		next.pixelFormat = 'auto';
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

	if (isGifOutput && !isCopyMode) {
		next.pixelFormat = 'auto';
		next.videoCodec = 'gif';
		next.videoBitrateMode = 'crf';
		next.mlUpscale = 'none';
		next.hwDecode = false;
		next.nvencSpatialAq = false;
		next.nvencTemporalAq = false;
		next.videotoolboxAllowSw = false;
	}

	if (!isCopyMode && !isAudioContainer && !isVideoCodecAllowed(next.container, next.videoCodec)) {
		next.videoCodec = getFirstAllowedVideoCodec(next.container);
	}

	if (
		!isCopyMode &&
		!isAudioContainer &&
		!isGifOutput &&
		!isVideoPixelFormatAllowed(next.container, next.videoCodec, next.pixelFormat ?? 'auto')
	) {
		next.pixelFormat = getFirstAllowedVideoPixelFormat(next.container, next.videoCodec);
	}

	if (
		!isCopyMode &&
		next.mlUpscale &&
		next.mlUpscale !== 'none' &&
		next.resolution !== 'original'
	) {
		next.resolution = 'original';
	}

	if (!isCopyMode && !isVideoPresetAllowed(next.videoCodec, next.preset)) {
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
