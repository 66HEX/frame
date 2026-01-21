export enum FileStatus {
	IDLE = 'IDLE',
	QUEUED = 'QUEUED',
	CONVERTING = 'CONVERTING',
	COMPLETED = 'COMPLETED',
	ERROR = 'ERROR'
}

export interface AudioTrack {
	index: number;
	codec: string;
	channels: string;
	language?: string;
	label?: string;
}

export interface ConversionConfig {
	container: string;
	videoCodec: string;
	videoBitrateMode: 'crf' | 'bitrate';
	videoBitrate: string;
	audioCodec: string;
	audioBitrate: string;
	audioChannels: 'original' | 'stereo' | 'mono';
	selectedAudioTracks: number[];
	resolution: string;
	customWidth?: string;
	customHeight?: string;
	scalingAlgorithm: 'bicubic' | 'lanczos' | 'bilinear' | 'nearest';
	fps: string;
	crf: number;
	preset: string;
}

export interface SourceMetadata {
	duration?: string;
	bitrate?: string;
	videoCodec?: string;
	audioCodec?: string;
	resolution?: string;
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

export interface SourceMetadata {
	duration?: string;
	bitrate?: string;
	videoCodec?: string;
	audioCodec?: string;
	resolution?: string;
	audioTracks?: AudioTrack[];
}

export type MetadataStatus = 'idle' | 'loading' | 'ready' | 'error';

export interface OutputEstimate {
	videoKbps: number;
	audioKbps: number;
	totalKbps: number;
	sizeMb?: number;
}
