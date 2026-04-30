import { convertFileSrc } from '@tauri-apps/api/core';
import { Assets, Texture } from 'pixi.js';

export async function loadPreviewTexture(filePath: string) {
	const assetUrl = convertFileSrc(filePath);
	const loaded = await Assets.load({
		src: assetUrl,
		data: {
			autoPlay: false,
			muted: false,
			loop: false,
			playsinline: true,
			preload: true
		}
	});

	if (!(loaded instanceof Texture)) {
		throw new Error('Pixi Assets.load did not return a Texture for preview media');
	}

	return { assetUrl, texture: loaded };
}

export async function unloadPreviewAsset(assetUrl: string) {
	try {
		await Assets.unload(assetUrl);
	} catch {
		// Ignore cache unload issues for local media assets.
	}
}

export function getPreviewAssetUrl(filePath: string) {
	return convertFileSrc(filePath);
}
