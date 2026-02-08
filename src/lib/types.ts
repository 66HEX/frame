export enum FileStatus {
	IDLE = 'IDLE',
	QUEUED = 'QUEUED',
	CONVERTING = 'CONVERTING',
	PAUSED = 'PAUSED',
	COMPLETED = 'COMPLETED',
	ERROR = 'ERROR'
}

export interface AudioTrack {
	index: number;
	codec: string;
	channels: string;
	language?: string;
	label?: string;
	bitrateKbps?: number;
	sampleRate?: string;
}

export interface SubtitleTrack {
	index: number;
	codec: string;
	language?: string;
	label?: string;
}

export interface CropSettings {
	enabled: boolean;
	x: number;
	y: number;
	width: number;
	height: number;
	sourceWidth?: number;
	sourceHeight?: number;
	aspectRatio?: string | null;
}

export interface ConversionConfig {
	container: string;
	videoCodec: string;
	videoBitrateMode: 'crf' | 'bitrate';
	videoBitrate: string;
	audioCodec: string;
	audioBitrate: string;
	audioChannels: string;
	audioVolume: number;
	audioNormalize: boolean;
	selectedAudioTracks: number[];
	selectedSubtitleTracks: number[];
	subtitleBurnPath?: string;
	resolution: string;
	customWidth?: string;
	customHeight?: string;
	scalingAlgorithm: 'bicubic' | 'lanczos' | 'bilinear' | 'nearest';
	fps: string;
	crf: number;
	quality: number;
	preset: string;
	startTime?: string;
	endTime?: string;
	metadata: MetadataConfig;
	rotation: '0' | '90' | '180' | '270';
	flipHorizontal: boolean;
	flipVertical: boolean;
	mlUpscale?: 'none' | 'esrgan-2x' | 'esrgan-4x';
	crop?: CropSettings | null;
	nvencSpatialAq: boolean;
	nvencTemporalAq: boolean;
	videotoolboxAllowSw: boolean;
	hwDecode: boolean;
}

export type MetadataMode = 'preserve' | 'clean' | 'replace';

export interface MetadataConfig {
	mode: MetadataMode;
	title?: string;
	artist?: string;
	album?: string;
	genre?: string;
	date?: string;
	comment?: string;
}

export interface SourceMetadata {
	duration?: string;
	bitrate?: string;
	videoCodec?: string;
	audioCodec?: string;
	resolution?: string;
	frameRate?: number;
	width?: number;
	height?: number;
	videoBitrateKbps?: number;
	audioTracks?: AudioTrack[];
	subtitleTracks?: SubtitleTrack[];
	tags?: Record<string, string>;
	pixelFormat?: string;
	colorSpace?: string;
	colorRange?: string;
	colorPrimaries?: string;
	profile?: string;
}

export interface FileItem {
	id: string;
	name: string;
	size: number;
	status: FileStatus;
	progress: number;
	originalFormat: string;
	config: ConversionConfig;
	outputName: string;
	metadata?: SourceMetadata;
	metadataStatus: MetadataStatus;
	metadataError?: string;
	path: string;
	isSelectedForConversion: boolean;
	conversionError?: string;
}

export interface PresetDefinition {
	id: string;
	name: string;
	config: ConversionConfig;
	builtIn?: boolean;
}

export type MetadataStatus = 'idle' | 'loading' | 'ready' | 'error';

export const AUDIO_ONLY_CONTAINERS = ['mp3', 'm4a', 'wav', 'flac'];

export const ALL_CONTAINERS = ['mp4', 'mkv', 'webm', 'mov', 'mp3', 'm4a', 'wav', 'flac'] as const;
