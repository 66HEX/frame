import { getAvailableEncoders, type AvailableEncoders } from '$lib/services/capabilities';

export const capabilities = $state<{ encoders: AvailableEncoders }>({
	encoders: {
		h264_videotoolbox: false,
		h264_nvenc: false,
		hevc_videotoolbox: false,
		hevc_nvenc: false,
		av1_nvenc: false
	}
});

export async function initCapabilities() {
	const encoders = await getAvailableEncoders();
	capabilities.encoders = encoders;
}
