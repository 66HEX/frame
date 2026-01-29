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
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
			resolution: 'original',
			scalingAlgorithm: 'lanczos',
			fps: 'original',
			crf: 18,
			quality: 60,
			preset: 'slow',
			startTime: undefined,
			endTime: undefined
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
			resolution: '720p',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 30,
			quality: 40,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
			resolution: 'original',
			scalingAlgorithm: 'bicubic',
			fps: 'original',
			crf: 23,
			quality: 50,
			preset: 'medium',
			startTime: undefined,
			endTime: undefined
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
