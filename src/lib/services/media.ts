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
