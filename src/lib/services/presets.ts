import { Store } from '@tauri-apps/plugin-store';
import { v4 as uuidv4 } from 'uuid';

import type { ConversionConfig, PresetDefinition } from '../types';

const STORE_PATH = 'presets.dat';
const PRESETS_KEY = 'presets';

export const DEFAULT_PRESETS: PresetDefinition[] = [
	{
		id: 'balanced-mp4',
		name: 'Balanced MP4',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'crf',
			videoBitrate: '5000',
			audioCodec: 'aac',
			audioBitrate: '128',
			audioChannels: 'original',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'archive-hq',
		name: 'Archive H.265',
		builtIn: true,
		config: {
			container: 'mkv',
			videoCodec: 'libx265',
			videoBitrateMode: 'crf',
			videoBitrate: '8000',
			audioCodec: 'ac3',
			audioBitrate: '192',
			audioChannels: 'original',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'lanczos',
			fps: 'original',
			crf: 18,
			quality: 60,
			preset: 'slow',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'web-share',
		name: 'Web Share',
		builtIn: true,
		config: {
			container: 'webm',
			videoCodec: 'vp9',
			videoBitrateMode: 'crf',
			videoBitrate: '2500',
			audioCodec: 'libopus',
			audioBitrate: '96',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: '720p',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 30,
			quality: 40,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'audio-only',
		name: 'Audio MP3',
		builtIn: true,
		config: {
			container: 'mp3',
			videoCodec: 'libx264',
			videoBitrateMode: 'crf',
			videoBitrate: '0',
			audioCodec: 'mp3',
			audioBitrate: '128',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'audio-flac',
		name: 'Audio FLAC (Lossless)',
		builtIn: true,
		config: {
			container: 'flac',
			videoCodec: 'libx264',
			videoBitrateMode: 'crf',
			videoBitrate: '0',
			audioCodec: 'flac',
			audioBitrate: '0',
			audioChannels: 'original',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'audio-alac',
		name: 'Audio ALAC (Apple)',
		builtIn: true,
		config: {
			container: 'm4a',
			videoCodec: 'libx264',
			videoBitrateMode: 'crf',
			videoBitrate: '0',
			audioCodec: 'alac',
			audioBitrate: '0',
			audioChannels: 'original',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'audio-wav',
		name: 'Audio WAV (Lossless)',
		builtIn: true,
		config: {
			container: 'wav',
			videoCodec: 'libx264',
			videoBitrateMode: 'crf',
			videoBitrate: '0',
			audioCodec: 'pcm_s16le',
			audioBitrate: '0',
			audioChannels: 'original',
			audioVolume: 100,
			audioNormalize: false,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'social-tiktok',
		name: 'Social (TikTok/Reels)',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '6000',
			audioCodec: 'aac',
			audioBitrate: '128',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'custom',
			customWidth: '1080',
			customHeight: '1920',
			scalingAlgorithm: 'lanczos',
			fps: '30',
			crf: 23,
			quality: 50,
			preset: 'slow',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'clean'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'yt-1080p',
		name: 'YouTube 1080p',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '10000',
			audioCodec: 'aac',
			audioBitrate: '320',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: '1080p',
			scalingAlgorithm: 'lanczos',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'slow',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'yt-4k',
		name: 'YouTube 4K',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '40000',
			audioCodec: 'aac',
			audioBitrate: '320',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'custom',
			customWidth: '3840',
			customHeight: '2160',
			scalingAlgorithm: 'lanczos',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'slow',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'preserve'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'x-landscape',
		name: 'X (Landscape)',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '2500',
			audioCodec: 'aac',
			audioBitrate: '128',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: '720p',
			scalingAlgorithm: 'lanczos',
			fps: '30',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'clean'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'x-portrait',
		name: 'X (Mobile/Portrait)',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '2000',
			audioCodec: 'aac',
			audioBitrate: '128',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: 'custom',
			customWidth: '720',
			customHeight: '1280',
			scalingAlgorithm: 'lanczos',
			fps: '30',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'clean'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	},
	{
		id: 'discord',
		name: 'Discord',
		builtIn: true,
		config: {
			container: 'mp4',
			videoCodec: 'libx264',
			videoBitrateMode: 'bitrate',
			videoBitrate: '1000',
			audioCodec: 'aac',
			audioBitrate: '64',
			audioChannels: 'stereo',
			audioVolume: 100,
			audioNormalize: true,
			selectedAudioTracks: [],
			selectedSubtitleTracks: [],
			subtitleBurnPath: undefined,
			resolution: '720p',
			scalingAlgorithm: 'bicubic',
			fps: '30',
			crf: 23,
			quality: 50,
			preset: 'veryfast',
			startTime: undefined,
			endTime: undefined,
			metadata: {
				mode: 'clean'
			},
			rotation: '0',
			flipHorizontal: false,
			flipVertical: false,
			crop: null,
			nvencSpatialAq: false,
			nvencTemporalAq: false,
			videotoolboxAllowSw: false,
			hwDecode: false
		}
	}
];

let storePromise: Promise<Store> | null = null;

async function getStore(): Promise<Store> {
	if (!storePromise) {
		storePromise = Store.load(STORE_PATH, {
			defaults: {
				[PRESETS_KEY]: []
			}
		});
	}

	return storePromise;
}

export async function loadCustomPresets(): Promise<PresetDefinition[]> {
	try {
		const store = await getStore();
		const presets = await store.get<PresetDefinition[]>(PRESETS_KEY);
		return presets ?? [];
	} catch (error) {
		console.error('Failed to load presets from store', error);
		return [];
	}
}

export async function saveCustomPresets(presets: PresetDefinition[]): Promise<void> {
	const store = await getStore();
	await store.set(PRESETS_KEY, presets);
	await store.save();
}

export function createCustomPreset(name: string, config: ConversionConfig): PresetDefinition {
	return {
		id: uuidv4(),
		name: name.trim() || 'Untitled Preset',
		config: cloneConfig(config)
	};
}

export function cloneConfig(config: ConversionConfig): ConversionConfig {
	return JSON.parse(JSON.stringify(config));
}

export function getDefaultConfig(): ConversionConfig {
	return cloneConfig(DEFAULT_PRESETS[0].config);
}
