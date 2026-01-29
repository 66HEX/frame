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
}

export interface ConversionConfig {
	container: string;
	videoCodec: string;
	videoBitrateMode: 'crf' | 'bitrate';
	videoBitrate: string;
	audioCodec: string;
	audioBitrate: string;
	audioChannels: 'original' | 'stereo' | 'mono';
	audioVolume: number;
	audioNormalize: boolean;
	selectedAudioTracks: number[];
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
