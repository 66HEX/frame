import { Application, Container, Graphics, Sprite, Texture } from 'pixi.js';
import { getPreviewAssetUrl, loadPreviewTexture, unloadPreviewAsset } from './previewAssets';
import { createPreviewPixiScene } from './previewPixiApp';
import {
	DEFAULT_PREVIEW_ZOOM,
	MAX_PREVIEW_ZOOM,
	MIN_PREVIEW_ZOOM,
	PREVIEW_TRANSFORM_EPSILON,
	PREVIEW_TRANSFORM_LERP,
	PREVIEW_ZOOM_STEP,
	clampPreviewTransform,
	clampValue,
	defaultPreviewTransform,
	getPreviewResolution,
	getSceneMetrics
} from './previewScene';
import type {
	PreviewMediaKind,
	PreviewPresentationState,
	PreviewSource,
	PreviewTransform
} from './previewTypes';
import { getHandleCursor, type DragHandle } from '$lib/utils/crop';

const CROP_BORDER_SCREEN_WIDTH = 1.25;
const CROP_GUIDE_SCREEN_WIDTH = 1;
const CROP_HANDLE_SCREEN_RADIUS = 5.5;
const CROP_HIT_SCREEN_RADIUS = 12;
const INITIAL_VIDEO_FRAME_PRIME_SECONDS = 0.001;
const INITIAL_VIDEO_FRAME_PRIME_TIMEOUT_MS = 700;

interface CropPointerTarget {
	handle: DragHandle;
	point: { x: number; y: number };
	cursor: string;
}

type PreviewVideoTextureSource = {
	resource?: unknown;
	update?: () => void;
	updateFrame?: () => void;
};

export function createPreviewRenderer() {
	let canvasElement = $state<HTMLCanvasElement | undefined>();
	let wrapperElement = $state<HTMLDivElement | undefined>();
	let app = $state<Application | null>(null);
	let appInitPromise: Promise<void> | null = null;
	let spriteContainer = $state<Container | null>(null);
	let rotationContainer = $state<Container | null>(null);
	let flipContainer = $state<Container | null>(null);
	let sprite = $state<Sprite | null>(null);
	let cropMask = $state<Graphics | null>(null);
	let cropOverlay = $state<Graphics | null>(null);
	let resizeObserver = $state<ResizeObserver | null>(null);
	let texture = $state<Texture | null>(null);
	let mediaElement = $state<HTMLVideoElement | undefined>();
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let frameLoopCleanup: (() => void) | null = null;
	let currentAssetUrl = $state<string | null>(null);
	let pendingSource: PreviewSource | null = null;
	let sourceRequestId = 0;
	let naturalWidth = $state(0);
	let naturalHeight = $state(0);
	let wrapperWidth = $state(0);
	let wrapperHeight = $state(0);
	let previewTransform = $state<PreviewTransform>(defaultPreviewTransform());
	let renderedPreviewTransform: PreviewTransform = defaultPreviewTransform();
	let previewTransformFrame = 0;
	let presentation = $state<PreviewPresentationState>({
		mediaKind: 'video',
		rotation: '0',
		flipHorizontal: false,
		flipVertical: false,
		cropMode: false,
		appliedCrop: null,
		draftCrop: null
	});

	function getSceneState() {
		return {
			presentation,
			naturalWidth,
			naturalHeight,
			wrapperWidth,
			wrapperHeight
		};
	}

	function resizeAppToWrapper() {
		if (!app || !wrapperWidth || !wrapperHeight) return;

		app.renderer.resize(
			Math.max(1, wrapperWidth),
			Math.max(1, wrapperHeight),
			getPreviewResolution()
		);
	}

	async function ensureApp(): Promise<boolean> {
		if (!canvasElement) return false;
		if (!wrapperElement || !wrapperWidth || !wrapperHeight) return false;
		if (app) return true;
		if (appInitPromise) {
			await appInitPromise;
			return Boolean(app);
		}

		appInitPromise = (async () => {
			const scene = await createPreviewPixiScene(
				canvasElement,
				wrapperWidth,
				wrapperHeight,
				getPreviewResolution()
			);

			app = scene.app;
			spriteContainer = scene.spriteContainer;
			rotationContainer = scene.rotationContainer;
			flipContainer = scene.flipContainer;
			sprite = scene.sprite;
			cropMask = scene.cropMask;
			cropOverlay = scene.cropOverlay;
			resizeAppToWrapper();
			updateScene();
		})();

		try {
			await appInitPromise;
		} finally {
			appInitPromise = null;
		}

		return Boolean(app);
	}

	async function clearTexture() {
		frameLoopCleanup?.();
		frameLoopCleanup = null;

		if (mediaElement) {
			mediaElement.pause();
		}

		if (currentAssetUrl) {
			await unloadPreviewAsset(currentAssetUrl);
		}

		texture?.destroy(false);
		texture = null;
		mediaElement = undefined;
		currentAssetUrl = null;
		naturalWidth = 0;
		naturalHeight = 0;

		if (sprite) {
			sprite.texture = Texture.EMPTY;
			sprite.visible = false;
		}

		cropMask?.clear();
		cropOverlay?.clear();
	}

	function getErrorMessage(cause: unknown) {
		return cause instanceof Error ? cause.message : 'Failed to load preview media';
	}

	function stopPreviewTransformAnimation() {
		if (!previewTransformFrame) return;
		window.cancelAnimationFrame(previewTransformFrame);
		previewTransformFrame = 0;
	}

	function isPreviewTransformSettled() {
		return (
			Math.abs(renderedPreviewTransform.zoom - previewTransform.zoom) <
				PREVIEW_TRANSFORM_EPSILON / 100 &&
			Math.abs(renderedPreviewTransform.offsetX - previewTransform.offsetX) <
				PREVIEW_TRANSFORM_EPSILON &&
			Math.abs(renderedPreviewTransform.offsetY - previewTransform.offsetY) <
				PREVIEW_TRANSFORM_EPSILON
		);
	}

	function startPreviewTransformAnimation() {
		if (previewTransformFrame) return;

		const tick = () => {
			previewTransformFrame = 0;

			renderedPreviewTransform = {
				zoom:
					renderedPreviewTransform.zoom +
					(previewTransform.zoom - renderedPreviewTransform.zoom) * PREVIEW_TRANSFORM_LERP,
				offsetX:
					renderedPreviewTransform.offsetX +
					(previewTransform.offsetX - renderedPreviewTransform.offsetX) * PREVIEW_TRANSFORM_LERP,
				offsetY:
					renderedPreviewTransform.offsetY +
					(previewTransform.offsetY - renderedPreviewTransform.offsetY) * PREVIEW_TRANSFORM_LERP
			};

			if (isPreviewTransformSettled()) {
				renderedPreviewTransform = previewTransform;
				updateScene();
				return;
			}

			updateScene();
			previewTransformFrame = window.requestAnimationFrame(tick);
		};

		previewTransformFrame = window.requestAnimationFrame(tick);
	}

	function applyPreviewTransform(nextTransform: PreviewTransform, immediate = false) {
		previewTransform = clampPreviewTransform(getSceneState(), nextTransform);

		if (immediate) {
			stopPreviewTransformAnimation();
			renderedPreviewTransform = previewTransform;
			updateScene();
			return;
		}

		startPreviewTransformAnimation();
	}

	function resetPreviewTransform(immediate = false) {
		applyPreviewTransform(defaultPreviewTransform(), immediate);
	}

	function syncNaturalDimensions() {
		if (!texture) return;
		const width = presentation.sourceWidth ?? texture.source.width;
		const height = presentation.sourceHeight ?? texture.source.height;
		naturalWidth = width || 0;
		naturalHeight = height || 0;
		updateScene();
	}

	function waitForVideoEvent(video: HTMLVideoElement, events: string[], timeoutMs: number) {
		return new Promise<void>((resolve) => {
			let timeoutId = 0;

			const cleanup = () => {
				if (timeoutId) {
					window.clearTimeout(timeoutId);
				}

				for (const event of events) {
					video.removeEventListener(event, done);
				}
			};
			const done = () => {
				cleanup();
				resolve();
			};

			for (const event of events) {
				video.addEventListener(event, done, { once: true });
			}

			timeoutId = window.setTimeout(done, timeoutMs);
		});
	}

	async function primeInitialVideoFrame(video: HTMLVideoElement) {
		if (mediaElement !== video) return;

		if (video.readyState < video.HAVE_CURRENT_DATA) {
			await waitForVideoEvent(
				video,
				['loadeddata', 'canplay', 'canplaythrough'],
				INITIAL_VIDEO_FRAME_PRIME_TIMEOUT_MS
			);
		}

		if (mediaElement !== video) return;

		const duration = Number.isFinite(video.duration) ? video.duration : 0;
		const canPrimeWithSeek =
			video.readyState >= video.HAVE_METADATA &&
			duration > INITIAL_VIDEO_FRAME_PRIME_SECONDS &&
			video.currentTime === 0;

		if (canPrimeWithSeek) {
			video.currentTime = INITIAL_VIDEO_FRAME_PRIME_SECONDS;
			await waitForVideoEvent(video, ['seeked'], INITIAL_VIDEO_FRAME_PRIME_TIMEOUT_MS);
		}

		if (mediaElement !== video) return;

		refreshVideoFrame(video);
		await new Promise<void>((resolve) => {
			window.requestAnimationFrame(() => {
				refreshVideoFrame(video);
				resolve();
			});
		});
	}

	async function bindTexture(nextTexture: Texture) {
		texture = nextTexture;
		if (nextTexture.source.resource instanceof HTMLVideoElement) {
			mediaElement = nextTexture.source.resource;
			mediaElement.muted = false;
			mediaElement.defaultMuted = false;
			mediaElement.volume = 1;
			mediaElement.loop = false;
			mediaElement.autoplay = false;
			mediaElement.playsInline = true;
			attachFrameLoop(mediaElement);
		} else {
			mediaElement = undefined;
		}

		if (sprite) {
			sprite.texture = nextTexture;
			sprite.visible = true;
		}

		syncNaturalDimensions();
		if (mediaElement) {
			await primeInitialVideoFrame(mediaElement);
		}
	}

	function refreshVideoFrame(video: HTMLVideoElement) {
		if (!app || mediaElement !== video) return;

		const source = texture?.source as PreviewVideoTextureSource | undefined;
		if (source?.resource === video) {
			if (typeof source.updateFrame === 'function') {
				source.updateFrame();
			} else if (typeof source.update === 'function') {
				source.update();
			}
		}

		app.render();
	}

	function attachFrameLoop(video: HTMLVideoElement) {
		frameLoopCleanup?.();
		frameLoopCleanup = null;

		if (typeof video.requestVideoFrameCallback === 'function') {
			let callbackId = 0;
			let pendingFrameCallbackId = 0;
			let pendingRenderFrame = 0;

			const onFrame = () => {
				if (!app || mediaElement !== video) return;
				refreshVideoFrame(video);
				callbackId = video.requestVideoFrameCallback(onFrame);
			};

			const start = () => {
				if (callbackId) return;
				callbackId = video.requestVideoFrameCallback(onFrame);
			};

			const stop = () => {
				if (!callbackId) return;
				video.cancelVideoFrameCallback(callbackId);
				callbackId = 0;
			};
			const cancelPendingFrameRefresh = () => {
				if (pendingFrameCallbackId) {
					video.cancelVideoFrameCallback(pendingFrameCallbackId);
					pendingFrameCallbackId = 0;
				}

				if (pendingRenderFrame) {
					window.cancelAnimationFrame(pendingRenderFrame);
					pendingRenderFrame = 0;
				}
			};
			const renderCurrentFrame = () => {
				refreshVideoFrame(video);

				if (!video.paused && !video.ended) return;

				if (pendingFrameCallbackId) {
					video.cancelVideoFrameCallback(pendingFrameCallbackId);
				}
				pendingFrameCallbackId = video.requestVideoFrameCallback(() => {
					pendingFrameCallbackId = 0;
					refreshVideoFrame(video);
				});

				if (!pendingRenderFrame) {
					pendingRenderFrame = window.requestAnimationFrame(() => {
						pendingRenderFrame = 0;
						refreshVideoFrame(video);
					});
				}
			};

			video.addEventListener('play', start);
			video.addEventListener('pause', stop);
			video.addEventListener('ended', stop);
			video.addEventListener('seeking', renderCurrentFrame);
			video.addEventListener('seeked', renderCurrentFrame);
			video.addEventListener('timeupdate', renderCurrentFrame);
			video.addEventListener('loadedmetadata', renderCurrentFrame);
			video.addEventListener('loadeddata', renderCurrentFrame);
			video.addEventListener('canplay', renderCurrentFrame);
			video.addEventListener('canplaythrough', renderCurrentFrame);

			if (!video.paused) {
				start();
			}

			frameLoopCleanup = () => {
				stop();
				cancelPendingFrameRefresh();
				video.removeEventListener('play', start);
				video.removeEventListener('pause', stop);
				video.removeEventListener('ended', stop);
				video.removeEventListener('seeking', renderCurrentFrame);
				video.removeEventListener('seeked', renderCurrentFrame);
				video.removeEventListener('timeupdate', renderCurrentFrame);
				video.removeEventListener('loadedmetadata', renderCurrentFrame);
				video.removeEventListener('loadeddata', renderCurrentFrame);
				video.removeEventListener('canplay', renderCurrentFrame);
				video.removeEventListener('canplaythrough', renderCurrentFrame);
			};
			return;
		}

		let rafId = 0;
		let pendingRenderFrame = 0;
		const tick = () => {
			if (!app || mediaElement !== video) return;
			refreshVideoFrame(video);
			if (!video.paused && !video.ended) {
				rafId = window.requestAnimationFrame(tick);
			}
		};
		const start = () => {
			if (rafId) return;
			rafId = window.requestAnimationFrame(tick);
		};
		const stop = () => {
			if (!rafId) return;
			window.cancelAnimationFrame(rafId);
			rafId = 0;
		};
		const cancelPendingFrameRefresh = () => {
			if (!pendingRenderFrame) return;
			window.cancelAnimationFrame(pendingRenderFrame);
			pendingRenderFrame = 0;
		};
		const renderCurrentFrame = () => {
			refreshVideoFrame(video);

			if (!video.paused && !video.ended) return;
			if (pendingRenderFrame) return;

			pendingRenderFrame = window.requestAnimationFrame(() => {
				pendingRenderFrame = 0;
				refreshVideoFrame(video);
			});
		};

		video.addEventListener('play', start);
		video.addEventListener('pause', stop);
		video.addEventListener('ended', stop);
		video.addEventListener('seeking', renderCurrentFrame);
		video.addEventListener('seeked', renderCurrentFrame);
		video.addEventListener('timeupdate', renderCurrentFrame);
		video.addEventListener('loadedmetadata', renderCurrentFrame);
		video.addEventListener('loadeddata', renderCurrentFrame);
		video.addEventListener('canplay', renderCurrentFrame);
		video.addEventListener('canplaythrough', renderCurrentFrame);

		if (!video.paused) {
			start();
		}

		frameLoopCleanup = () => {
			stop();
			cancelPendingFrameRefresh();
			video.removeEventListener('play', start);
			video.removeEventListener('pause', stop);
			video.removeEventListener('ended', stop);
			video.removeEventListener('seeking', renderCurrentFrame);
			video.removeEventListener('seeked', renderCurrentFrame);
			video.removeEventListener('timeupdate', renderCurrentFrame);
			video.removeEventListener('loadedmetadata', renderCurrentFrame);
			video.removeEventListener('loadeddata', renderCurrentFrame);
			video.removeEventListener('canplay', renderCurrentFrame);
			video.removeEventListener('canplaythrough', renderCurrentFrame);
		};
	}

	async function setSource(filePath: string, mediaKind: PreviewMediaKind) {
		const requestId = ++sourceRequestId;
		pendingSource = { filePath, mediaKind };
		presentation = { ...presentation, mediaKind };
		error = null;

		if (mediaKind === 'unknown' || mediaKind === 'audio') {
			pendingSource = null;
			isLoading = false;
			await clearTexture();
			resetPreviewTransform(true);
			return;
		}

		if (!canvasElement || !wrapperElement || !wrapperWidth || !wrapperHeight) {
			isLoading = true;
			return;
		}

		const hasApp = await ensureApp();
		if (!hasApp) return;
		if (requestId !== sourceRequestId) return;

		const assetUrl = getPreviewAssetUrl(filePath);
		if (currentAssetUrl === assetUrl && texture) {
			pendingSource = null;
			isLoading = false;
			return;
		}

		isLoading = true;
		try {
			await clearTexture();
			resetPreviewTransform(true);
			if (requestId !== sourceRequestId) return;

			const { texture: loaded } = await loadPreviewTexture(filePath);

			if (requestId !== sourceRequestId) {
				const latestAssetUrl = pendingSource ? getPreviewAssetUrl(pendingSource.filePath) : null;
				if (latestAssetUrl !== assetUrl && currentAssetUrl !== assetUrl) {
					loaded.destroy(false);
					await unloadPreviewAsset(assetUrl);
				}
				return;
			}

			currentAssetUrl = assetUrl;
			pendingSource = null;
			await bindTexture(loaded);
			if (requestId !== sourceRequestId) return;
			isLoading = false;
		} catch (cause) {
			if (requestId !== sourceRequestId) return;
			isLoading = false;
			error = getErrorMessage(cause);
			pendingSource = null;
			await clearTexture();
			resetPreviewTransform(true);
		}
	}

	function retryPendingSource() {
		if (!pendingSource || pendingSource.mediaKind === 'audio') return;
		if (!canvasElement || !wrapperElement) return;

		const { filePath, mediaKind } = pendingSource;
		void setSource(filePath, mediaKind);
	}

	function setPresentationState(nextPresentation: PreviewPresentationState) {
		const shouldResetView =
			presentation.mediaKind !== nextPresentation.mediaKind ||
			presentation.sourceWidth !== nextPresentation.sourceWidth ||
			presentation.sourceHeight !== nextPresentation.sourceHeight;

		if (shouldResetView) {
			previewTransform = defaultPreviewTransform();
			renderedPreviewTransform = defaultPreviewTransform();
			stopPreviewTransformAnimation();
		}

		presentation = nextPresentation;
		syncNaturalDimensions();
		updateScene();
	}

	function setCanvasElement(element?: HTMLCanvasElement) {
		canvasElement = element;
		void ensureApp().then(retryPendingSource);
	}

	function setWrapperElement(element?: HTMLDivElement) {
		if (wrapperElement === element) return;
		resizeObserver?.disconnect();
		wrapperElement = element;

		if (!wrapperElement) return;

		const updateWrapperSize = () => {
			if (!wrapperElement) return;
			const rect = wrapperElement.getBoundingClientRect();
			wrapperWidth = rect.width;
			wrapperHeight = rect.height;
			if (app) {
				resizeAppToWrapper();
			}
			updateScene();
			void ensureApp().then(retryPendingSource);
		};

		resizeObserver = new ResizeObserver(updateWrapperSize);
		resizeObserver.observe(wrapperElement);
		updateWrapperSize();
		void ensureApp().then(retryPendingSource);
	}

	function setPreviewTransform(nextTransform: PreviewTransform) {
		applyPreviewTransform(nextTransform);
	}

	function zoomPreviewAt(clientX: number, clientY: number, deltaY: number) {
		if (!wrapperElement) return;

		const rect = wrapperElement.getBoundingClientRect();
		if (
			clientX < rect.left ||
			clientX > rect.right ||
			clientY < rect.top ||
			clientY > rect.bottom
		) {
			return;
		}

		const oldZoom = previewTransform.zoom;
		const nextZoom = clampValue(
			oldZoom * (deltaY < 0 ? PREVIEW_ZOOM_STEP : 1 / PREVIEW_ZOOM_STEP),
			MIN_PREVIEW_ZOOM,
			MAX_PREVIEW_ZOOM
		);

		if (nextZoom === oldZoom) return;

		const ratio = nextZoom / oldZoom;
		const pointerX = clientX - rect.left - wrapperWidth / 2;
		const pointerY = clientY - rect.top - wrapperHeight / 2;

		setPreviewTransform({
			zoom: nextZoom,
			offsetX: pointerX - (pointerX - previewTransform.offsetX) * ratio,
			offsetY: pointerY - (pointerY - previewTransform.offsetY) * ratio
		});
	}

	function zoomPreviewBy(direction: 1 | -1) {
		const oldZoom = previewTransform.zoom;
		const nextZoom = clampValue(
			oldZoom * (direction > 0 ? PREVIEW_ZOOM_STEP : 1 / PREVIEW_ZOOM_STEP),
			MIN_PREVIEW_ZOOM,
			MAX_PREVIEW_ZOOM
		);
		const ratio = nextZoom / oldZoom;

		setPreviewTransform({
			zoom: nextZoom,
			offsetX: previewTransform.offsetX * ratio,
			offsetY: previewTransform.offsetY * ratio
		});
	}

	function panPreviewBy(deltaX: number, deltaY: number) {
		setPreviewTransform({
			zoom: previewTransform.zoom,
			offsetX: previewTransform.offsetX + deltaX,
			offsetY: previewTransform.offsetY + deltaY
		});
	}

	function getRenderTransform() {
		return renderedPreviewTransform;
	}

	function getVisualDimensions() {
		const baseWidth = presentation.sourceWidth ?? naturalWidth;
		const baseHeight = presentation.sourceHeight ?? naturalHeight;
		if (!baseWidth || !baseHeight) return null;

		const sideRotation = presentation.rotation === '90' || presentation.rotation === '270';
		return {
			width: sideRotation ? baseHeight : baseWidth,
			height: sideRotation ? baseWidth : baseHeight,
			sideRotation
		};
	}

	function getCropLocalPoint(clientX: number, clientY: number, allowOutside = false) {
		if (!wrapperElement) return null;

		const transform = getRenderTransform();
		const metrics = getSceneMetrics(getSceneState(), transform.zoom);
		if (!metrics) return null;

		const rect = wrapperElement.getBoundingClientRect();
		if (
			!allowOutside &&
			(clientX < rect.left || clientX > rect.right || clientY < rect.top || clientY > rect.bottom)
		) {
			return null;
		}

		const scale = metrics.fitScale * transform.zoom;
		if (!scale) return null;

		const visual = getVisualDimensions();
		if (!visual) return null;

		const localX = (clientX - rect.left - wrapperWidth / 2 - transform.offsetX) / scale;
		const localY = (clientY - rect.top - wrapperHeight / 2 - transform.offsetY) / scale;

		return {
			localX,
			localY,
			scale,
			visualWidth: visual.width,
			visualHeight: visual.height,
			sideRotation: visual.sideRotation,
			point: {
				x: clampValue((localX + visual.width / 2) / visual.width, 0, 1),
				y: clampValue((localY + visual.height / 2) / visual.height, 0, 1)
			}
		};
	}

	function getCropPointerTarget(clientX: number, clientY: number): CropPointerTarget | null {
		if (!presentation.cropMode || !presentation.draftCrop) return null;

		const local = getCropLocalPoint(clientX, clientY);
		if (!local) return null;

		const { draftCrop } = presentation;
		const left = (draftCrop.x - 0.5) * local.visualWidth;
		const right = (draftCrop.x + draftCrop.width - 0.5) * local.visualWidth;
		const top = (draftCrop.y - 0.5) * local.visualHeight;
		const bottom = (draftCrop.y + draftCrop.height - 0.5) * local.visualHeight;
		const centerX = (left + right) / 2;
		const centerY = (top + bottom) / 2;
		const threshold = CROP_HIT_SCREEN_RADIUS / local.scale;

		const handles: Array<{ handle: DragHandle; x: number; y: number }> = [
			{ handle: 'nw', x: left, y: top },
			{ handle: 'n', x: centerX, y: top },
			{ handle: 'ne', x: right, y: top },
			{ handle: 'e', x: right, y: centerY },
			{ handle: 'se', x: right, y: bottom },
			{ handle: 's', x: centerX, y: bottom },
			{ handle: 'sw', x: left, y: bottom },
			{ handle: 'w', x: left, y: centerY }
		];

		for (const { handle, x, y } of handles) {
			if (Math.hypot(local.localX - x, local.localY - y) <= threshold) {
				return {
					handle,
					point: local.point,
					cursor: getHandleCursor(handle, false)
				};
			}
		}

		const nearHorizontal =
			local.localX >= left - threshold &&
			local.localX <= right + threshold &&
			(Math.abs(local.localY - top) <= threshold || Math.abs(local.localY - bottom) <= threshold);
		const nearVertical =
			local.localY >= top - threshold &&
			local.localY <= bottom + threshold &&
			(Math.abs(local.localX - left) <= threshold || Math.abs(local.localX - right) <= threshold);

		if (nearHorizontal || nearVertical) {
			return { handle: 'move', point: local.point, cursor: 'move' };
		}

		const inside =
			local.localX >= left &&
			local.localX <= right &&
			local.localY >= top &&
			local.localY <= bottom;

		if (inside) {
			return { handle: 'move', point: local.point, cursor: 'move' };
		}

		return null;
	}

	function getCropPoint(clientX: number, clientY: number) {
		return getCropLocalPoint(clientX, clientY, true)?.point ?? null;
	}

	function drawCropOverlay(metrics: NonNullable<ReturnType<typeof getSceneMetrics>>) {
		if (!cropOverlay) return;

		cropOverlay.clear();
		if (!presentation.cropMode || !presentation.draftCrop) return;

		const visual = getVisualDimensions();
		if (!visual) return;

		const scale = metrics.fitScale * getRenderTransform().zoom;
		if (!scale) return;

		const { draftCrop } = presentation;
		const left = (draftCrop.x - 0.5) * visual.width;
		const top = (draftCrop.y - 0.5) * visual.height;
		const width = draftCrop.width * visual.width;
		const height = draftCrop.height * visual.height;
		const right = left + width;
		const bottom = top + height;
		const minX = -visual.width / 2;
		const minY = -visual.height / 2;
		const lineWidth = CROP_BORDER_SCREEN_WIDTH / scale;
		const guideWidth = CROP_GUIDE_SCREEN_WIDTH / scale;
		const handleRadius = CROP_HANDLE_SCREEN_RADIUS / scale;

		cropOverlay
			.rect(minX, minY, visual.width, top - minY)
			.rect(minX, top, left - minX, height)
			.rect(right, top, minX + visual.width - right, height)
			.rect(minX, bottom, visual.width, minY + visual.height - bottom)
			.fill({ color: 0x000000, alpha: 0.55 });

		cropOverlay
			.rect(left, top, width, height)
			.stroke({ color: 0xffffff, alpha: 0.9, width: lineWidth });

		for (let index = 1; index <= 2; index += 1) {
			const x = left + (width * index) / 3;
			const y = top + (height * index) / 3;
			cropOverlay
				.moveTo(x, top)
				.lineTo(x, bottom)
				.moveTo(left, y)
				.lineTo(right, y)
				.stroke({ color: 0xffffff, alpha: 0.7, width: guideWidth });
		}

		const handlePoints = [
			[left, top],
			[left + width / 2, top],
			[right, top],
			[right, top + height / 2],
			[right, bottom],
			[left + width / 2, bottom],
			[left, bottom],
			[left, top + height / 2]
		] as const;

		for (const [x, y] of handlePoints) {
			cropOverlay
				.circle(x, y, handleRadius)
				.fill(0xffffff)
				.stroke({ color: 0x000000, alpha: 0.45, width: lineWidth });
		}
	}

	function updateScene() {
		if (
			!app ||
			!spriteContainer ||
			!rotationContainer ||
			!flipContainer ||
			!sprite ||
			!texture ||
			!cropMask ||
			!cropOverlay
		)
			return;

		const effectiveTransform = clampPreviewTransform(getSceneState(), previewTransform);
		if (
			effectiveTransform.zoom !== previewTransform.zoom ||
			effectiveTransform.offsetX !== previewTransform.offsetX ||
			effectiveTransform.offsetY !== previewTransform.offsetY
		) {
			previewTransform = effectiveTransform;
		}

		renderedPreviewTransform = clampPreviewTransform(getSceneState(), renderedPreviewTransform);

		const renderTransform = renderedPreviewTransform;
		const metrics = getSceneMetrics(getSceneState(), renderTransform.zoom);
		if (!metrics) return;

		sprite.texture = texture;
		sprite.width = metrics.baseWidth;
		sprite.height = metrics.baseHeight;
		sprite.position.set(
			-(metrics.cropCenterX * metrics.baseWidth),
			-(metrics.cropCenterY * metrics.baseHeight)
		);
		sprite.visible = true;

		spriteContainer.position.set(
			wrapperWidth / 2 + renderTransform.offsetX,
			wrapperHeight / 2 + renderTransform.offsetY
		);
		spriteContainer.scale.set(
			metrics.fitScale * renderTransform.zoom,
			metrics.fitScale * renderTransform.zoom
		);
		rotationContainer.rotation = (Number(presentation.rotation) * Math.PI) / 180;
		flipContainer.scale.set(
			presentation.flipHorizontal ? -1 : 1,
			presentation.flipVertical ? -1 : 1
		);

		cropMask.clear();
		if (!presentation.cropMode && presentation.appliedCrop) {
			cropMask
				.rect(
					wrapperWidth / 2 + renderTransform.offsetX - metrics.displayedWidth / 2,
					wrapperHeight / 2 + renderTransform.offsetY - metrics.displayedHeight / 2,
					metrics.displayedWidth,
					metrics.displayedHeight
				)
				.fill(0xffffff);
			spriteContainer.mask = cropMask;
		} else {
			spriteContainer.mask = null;
		}

		drawCropOverlay(metrics);
		app.render();
	}

	function destroy() {
		sourceRequestId += 1;
		stopPreviewTransformAnimation();
		resizeObserver?.disconnect();
		void clearTexture();
		app?.destroy(true, { children: true, texture: true });
		app = null;
		spriteContainer = null;
		rotationContainer = null;
		flipContainer = null;
		sprite = null;
		cropMask = null;
		cropOverlay = null;
	}

	return {
		get mediaElement() {
			return mediaElement;
		},
		get naturalWidth() {
			return naturalWidth;
		},
		get naturalHeight() {
			return naturalHeight;
		},
		get previewZoom() {
			return previewTransform.zoom;
		},
		get hasPreviewTransform() {
			return (
				previewTransform.zoom !== DEFAULT_PREVIEW_ZOOM ||
				previewTransform.offsetX !== 0 ||
				previewTransform.offsetY !== 0
			);
		},
		get isLoading() {
			return isLoading;
		},
		get error() {
			return error;
		},
		setCanvasElement,
		setWrapperElement,
		setSource,
		setPresentationState,
		getCropPointerTarget,
		getCropPoint,
		zoomPreviewAt,
		zoomPreviewBy,
		panPreviewBy,
		resetPreviewTransform,
		destroy
	};
}

export type PreviewRendererController = ReturnType<typeof createPreviewRenderer>;
