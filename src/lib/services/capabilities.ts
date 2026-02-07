import { invoke } from '@tauri-apps/api/core';

export interface AvailableEncoders {
	h264_videotoolbox: boolean;
	h264_nvenc: boolean;
	hevc_videotoolbox: boolean;
	hevc_nvenc: boolean;
	av1_nvenc: boolean;
}

export async function getAvailableEncoders(): Promise<AvailableEncoders> {
	try {
		return await invoke('get_available_encoders');
	} catch (error) {
		console.error('Failed to get available encoders:', error);
		return {
			h264_videotoolbox: false,
			h264_nvenc: false,
			hevc_videotoolbox: false,
			hevc_nvenc: false,
			av1_nvenc: false
		};
	}
}
