import { invoke } from '@tauri-apps/api/core';
import type { SourceMetadata } from '$lib/types';
import {
	getDefaultAudioCodecForContainer,
	isAudioCodecAllowedForContainer
} from '$lib/constants/media-rules';

export async function probeMedia(filePath: string): Promise<SourceMetadata> {
	return invoke('probe_media', { filePath });
}

export function isAudioCodecAllowed(codec: string, container: string): boolean {
	return isAudioCodecAllowedForContainer(container, codec);
}

export function getDefaultAudioCodec(container: string): string {
	return getDefaultAudioCodecForContainer(container);
}

/**
 * Single source of truth (frontend) for which audio codecs expose a quality /
 * VBR mode in the UI. Mirrors `audio_codec_supports_vbr` in
 * `src-tauri/src/conversion/codec.rs`.
 */
export const VBR_CAPABLE_AUDIO_CODECS = ['mp3', 'libfdk_aac'] as const;

export function audioCodecSupportsVbr(codec: string): boolean {
	return (VBR_CAPABLE_AUDIO_CODECS as readonly string[]).includes(codec);
}

export interface AudioQualityRange {
	min: number;
	max: number;
	/** lower numeric value = better quality */
	lowerIsBetter: boolean;
	/** default slider position */
	defaultValue: number;
}

export function getAudioQualityRange(codec: string): AudioQualityRange | null {
	switch (codec) {
		case 'mp3':
			// libmp3lame -q:a 0..9 (0 = best, ~245 kbps; 9 = worst, ~65 kbps)
			return { min: 0, max: 9, lowerIsBetter: true, defaultValue: 4 };
		case 'libfdk_aac':
			// libfdk_aac -vbr 1..5 (1 = ~32 kbps/ch; 5 = ~112 kbps/ch)
			return { min: 1, max: 5, lowerIsBetter: false, defaultValue: 4 };
		default:
			return null;
	}
}
