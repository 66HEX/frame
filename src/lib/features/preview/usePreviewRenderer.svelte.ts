import { Application, Container, Graphics, Sprite, Texture } from 'pixi.js';
import { getPreviewAssetUrl, loadPreviewTexture, unloadPreviewAsset } from './previewAssets';
import { createPreviewPixiScene } from './previewPixiApp';
import {
	DEFAULT_PREVIEW_ZOOM,
	MAX_PREVIEW_ZOOM,
	MIN_PREVIEW_ZOOM,
	PREVIEW_BUTTON_ZOOM_STEP,
	PREVIEW_TRANSFORM_EPSILON,
	PREVIEW_TRANSFORM_LERP,
	PREVIEW_WHEEL_ZOOM_STEP,
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
import { MAX_OVERLAY_WIDTH } from './usePreviewOverlay.svelte';
import { getHandleCursor, type DragHandle } from '$lib/utils/crop';

const CROP_BORDER_SCREEN_WIDTH = 1.25;
const CROP_GUIDE_SCREEN_WIDTH = 1;
const CROP_HANDLE_SCREEN_RADIUS = 5.5;
const CROP_HIT_SCREEN_RADIUS = 12;
const OVERLAY_BORDER_SCREEN_WIDTH = 1.25;
const OVERLAY_HANDLE_SCREEN_RADIUS = 5.5;
const OVERLAY_HIT_SCREEN_SIZE = 16;
const INITIAL_VIDEO_FRAME_PRIME_SECONDS = 0.001;
const INITIAL_VIDEO_FRAME_PRIME_TIMEOUT_MS = 700;

interface CropPointerTarget {
	handle: DragHandle;
	point: { x: number; y: number };
	cursor: string;
}

interface OverlayPointerTarget {
	handle: 'move' | 'nw' | 'ne' | 'se' | 'sw';
	point: { x: number; y: number; width: number; height: number };
	cursor: string;
}

type PreviewVideoTextureSource = {
	resource?: unknown;
	update?: () => void;
	updateFrame?: () => void;
};

export function createPreviewRenderer() {
	let canvasElement: HTMLCanvasElement | undefined;
	let wrapperElement: HTMLDivElement | undefined;
	let app: Application | null = null;
	let appInitPromise: Promise<void> | null = null;
	let spriteContainer: Container | null = null;
	let rotationContainer: Container | null = null;
	let flipContainer: Container | null = null;
	let sprite: Sprite | null = null;
	let cropMask: Graphics | null = null;
	let cropOverlay: Graphics | null = null;
	let overlaySprite: Sprite | null = null;
	let overlayControls: Graphics | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let texture: Texture | null = null;
	let overlayTexture: Texture | null = null;
	let mediaElement = $state<HTMLVideoElement | undefined>();
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let frameLoopCleanup: (() => void) | null = null;
	let currentAssetUrl: string | null = null;
	let currentOverlayAssetUrl: string | null = null;
	let pendingSource: PreviewSource | null = null;
	let sourceRequestId = 0;
	let overlayRequestId = 0;
	let naturalWidth = $state(0);
	let naturalHeight = $state(0);
	let wrapperWidth = 0;
	let wrapperHeight = 0;
	let previewTransform = $state<PreviewTransform>(defaultPreviewTransform());
	let renderedPreviewTransform: PreviewTransform = defaultPreviewTransform();
	let previewTransformFrame = 0;
	let renderFrame = 0;
	let presentation: PreviewPresentationState = {
		mediaKind: 'video',
		rotation: '0',
		flipHorizontal: false,
		flipVertical: false,
		cropMode: false,
		appliedCrop: null,
		draftCrop: null,
		overlayMode: false,
		overlay: null
	};

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

	function cancelScheduledRender() {
		if (!renderFrame) return;
		window.cancelAnimationFrame(renderFrame);
		renderFrame = 0;
	}

	function requestRender() {
		if (!app || renderFrame) return;
		renderFrame = window.requestAnimationFrame(() => {
			renderFrame = 0;
			app?.render();
		});
	}

	function renderCurrentScene() {
		if (!app) return;
		cancelScheduledRender();
		app.render();
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
			overlaySprite = scene.overlaySprite;
			overlayControls = scene.overlayControls;
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

		const assetUrl = currentAssetUrl;
		const previousTexture = texture;
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
		if (overlaySprite) {
			overlaySprite.visible = false;
		}
		overlayControls?.clear();
		renderCurrentScene();

		if (assetUrl) {
			await unloadPreviewAsset(assetUrl);
		}

		previousTexture?.destroy(false);
	}

	async function clearOverlayTexture() {
		const assetUrl = currentOverlayAssetUrl;
		const previousTexture = overlayTexture;
		overlayTexture = null;
		currentOverlayAssetUrl = null;

		if (overlaySprite) {
			overlaySprite.texture = Texture.EMPTY;
			overlaySprite.visible = false;
		}

		overlayControls?.clear();
		renderCurrentScene();

		if (assetUrl) {
			await unloadPreviewAsset(assetUrl);
		}

		previousTexture?.destroy(false);
	}

	function getErrorMessage(cause: unknown) {
		return cause instanceof Error ? cause.message : 'Failed to load preview media';
	}

	function areCropRectsEqual(
		left: PreviewPresentationState['appliedCrop'],
		right: PreviewPresentationState['appliedCrop']
	) {
		if (left === right) return true;
		if (!left || !right) return false;
		return (
			left.x === right.x &&
			left.y === right.y &&
			left.width === right.width &&
			left.height === right.height
		);
	}

	function areOverlaysEqual(
		left: PreviewPresentationState['overlay'],
		right: PreviewPresentationState['overlay']
	) {
		if (left === right) return true;
		if (!left || !right) return false;
		return (
			left.enabled === right.enabled &&
			left.path === right.path &&
			left.x === right.x &&
			left.y === right.y &&
			left.width === right.width &&
			left.opacity === right.opacity &&
			left.anchor === right.anchor
		);
	}

	function arePresentationStatesEqual(
		left: PreviewPresentationState,
		right: PreviewPresentationState
	) {
		return (
			left.mediaKind === right.mediaKind &&
			left.rotation === right.rotation &&
			left.flipHorizontal === right.flipHorizontal &&
			left.flipVertical === right.flipVertical &&
			left.cropMode === right.cropMode &&
			left.overlayMode === right.overlayMode &&
			left.sourceWidth === right.sourceWidth &&
			left.sourceHeight === right.sourceHeight &&
			areCropRectsEqual(left.appliedCrop, right.appliedCrop) &&
			areCropRectsEqual(left.draftCrop, right.draftCrop) &&
			areOverlaysEqual(left.overlay, right.overlay)
		);
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

	async function syncOverlayTexture() {
		const requestId = ++overlayRequestId;
		const nextOverlay = presentation.overlay;
		if (!nextOverlay?.enabled || !nextOverlay.path || !app) {
			await clearOverlayTexture();
			updateScene();
			return;
		}

		const assetUrl = getPreviewAssetUrl(nextOverlay.path);
		if (currentOverlayAssetUrl === assetUrl && overlayTexture) {
			updateScene();
			return;
		}

		try {
			const { texture: loaded } = await loadPreviewTexture(nextOverlay.path);
			if (requestId !== overlayRequestId) {
				loaded.destroy(false);
				await unloadPreviewAsset(assetUrl);
				return;
			}

			await clearOverlayTexture();
			currentOverlayAssetUrl = assetUrl;
			overlayTexture = loaded;
			if (overlaySprite) {
				overlaySprite.texture = loaded;
			}
			updateScene();
		} catch (cause) {
			if (requestId !== overlayRequestId) return;
			console.error('Failed to load overlay image', cause);
			await clearOverlayTexture();
			updateScene();
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

		renderCurrentScene();
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
			const renderSettledMediaFrame = () => {
				if (!video.paused && !video.ended) return;
				renderCurrentFrame();
			};

			video.addEventListener('play', start);
			video.addEventListener('pause', stop);
			video.addEventListener('ended', stop);
			video.addEventListener('seeking', renderCurrentFrame);
			video.addEventListener('seeked', renderCurrentFrame);
			video.addEventListener('timeupdate', renderSettledMediaFrame);
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
				video.removeEventListener('timeupdate', renderSettledMediaFrame);
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
		const renderSettledMediaFrame = () => {
			if (!video.paused && !video.ended) return;
			renderCurrentFrame();
		};

		video.addEventListener('play', start);
		video.addEventListener('pause', stop);
		video.addEventListener('ended', stop);
		video.addEventListener('seeking', renderCurrentFrame);
		video.addEventListener('seeked', renderCurrentFrame);
		video.addEventListener('timeupdate', renderSettledMediaFrame);
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
			video.removeEventListener('timeupdate', renderSettledMediaFrame);
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

	function syncOverlayAfterAppReady() {
		if (!presentation.overlay?.enabled || !presentation.overlay.path) return;
		void syncOverlayTexture();
	}

	function setPresentationState(nextPresentation: PreviewPresentationState) {
		if (arePresentationStatesEqual(presentation, nextPresentation)) {
			if (
				nextPresentation.overlay?.enabled &&
				nextPresentation.overlay.path &&
				app &&
				!overlayTexture
			) {
				void syncOverlayTexture();
			}
			return;
		}

		const shouldResetView =
			presentation.mediaKind !== nextPresentation.mediaKind ||
			presentation.sourceWidth !== nextPresentation.sourceWidth ||
			presentation.sourceHeight !== nextPresentation.sourceHeight;

		if (shouldResetView) {
			previewTransform = defaultPreviewTransform();
			renderedPreviewTransform = defaultPreviewTransform();
			stopPreviewTransformAnimation();
		}

		const previousOverlayPath = presentation.overlay?.path;
		const nextOverlayPath = nextPresentation.overlay?.path;
		const shouldSyncOverlay =
			previousOverlayPath !== nextOverlayPath ||
			presentation.overlay?.enabled !== nextPresentation.overlay?.enabled;

		presentation = nextPresentation;
		syncNaturalDimensions();
		if (shouldSyncOverlay) {
			void ensureApp().then(syncOverlayTexture);
		}
		updateScene();
	}

	function setCanvasElement(element?: HTMLCanvasElement) {
		canvasElement = element;
		void ensureApp().then(() => {
			retryPendingSource();
			syncOverlayAfterAppReady();
		});
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
			void ensureApp().then(() => {
				retryPendingSource();
				syncOverlayAfterAppReady();
			});
		};

		resizeObserver = new ResizeObserver(updateWrapperSize);
		resizeObserver.observe(wrapperElement);
		updateWrapperSize();
		void ensureApp().then(() => {
			retryPendingSource();
			syncOverlayAfterAppReady();
		});
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
			oldZoom * (deltaY < 0 ? PREVIEW_WHEEL_ZOOM_STEP : 1 / PREVIEW_WHEEL_ZOOM_STEP),
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
			oldZoom * (direction > 0 ? PREVIEW_BUTTON_ZOOM_STEP : 1 / PREVIEW_BUTTON_ZOOM_STEP),
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

	function getOverlayRect() {
		if (!presentation.overlay?.enabled || !overlayTexture) return null;
		const transform = getRenderTransform();
		const metrics = getSceneMetrics(getSceneState(), transform.zoom);
		if (!metrics) return null;

		const sourceWidth = overlayTexture.source.width || 1;
		const sourceHeight = overlayTexture.source.height || 1;
		const sourceAspect = sourceHeight / sourceWidth;
		const maxWidth = Math.min(
			metrics.displayedWidth * MAX_OVERLAY_WIDTH,
			metrics.displayedHeight / sourceAspect
		);
		const width = Math.min(metrics.displayedWidth * presentation.overlay.width, maxWidth);
		const height = width * sourceAspect;
		const normalizedWidth = width / metrics.displayedWidth;
		const normalizedHeight = height / metrics.displayedHeight;
		const halfNormalizedWidth = Math.min(normalizedWidth / 2, 0.5);
		const halfNormalizedHeight = Math.min(normalizedHeight / 2, 0.5);
		const centerX = clampValue(
			presentation.overlay.x,
			halfNormalizedWidth,
			1 - halfNormalizedWidth
		);
		const centerY = clampValue(
			presentation.overlay.y,
			halfNormalizedHeight,
			1 - halfNormalizedHeight
		);
		const left =
			wrapperWidth / 2 +
			transform.offsetX -
			metrics.displayedWidth / 2 +
			centerX * metrics.displayedWidth -
			width / 2;
		const top =
			wrapperHeight / 2 +
			transform.offsetY -
			metrics.displayedHeight / 2 +
			centerY * metrics.displayedHeight -
			height / 2;

		return {
			left,
			top,
			width,
			height,
			right: left + width,
			bottom: top + height,
			centerX: left + width / 2,
			centerY: top + height / 2,
			frameLeft: wrapperWidth / 2 + transform.offsetX - metrics.displayedWidth / 2,
			frameTop: wrapperHeight / 2 + transform.offsetY - metrics.displayedHeight / 2,
			frameWidth: metrics.displayedWidth,
			frameHeight: metrics.displayedHeight
		};
	}

	function getOverlayPoint(clientX: number, clientY: number) {
		const rect = getOverlayRect();
		if (!rect || !wrapperElement) return null;

		const wrapperRect = wrapperElement.getBoundingClientRect();
		const localX = clientX - wrapperRect.left;
		const localY = clientY - wrapperRect.top;

		return {
			x: clampValue((localX - rect.frameLeft) / rect.frameWidth, 0, 1),
			y: clampValue((localY - rect.frameTop) / rect.frameHeight, 0, 1),
			width: rect.width / rect.frameWidth,
			height: rect.height / rect.frameHeight
		};
	}

	function getOverlayPointerTarget(clientX: number, clientY: number): OverlayPointerTarget | null {
		if (!presentation.overlayMode || !presentation.overlay?.enabled) return null;
		const rect = getOverlayRect();
		const point = getOverlayPoint(clientX, clientY);
		if (!rect || !point || !wrapperElement) return null;

		const wrapperRect = wrapperElement.getBoundingClientRect();
		const localX = clientX - wrapperRect.left;
		const localY = clientY - wrapperRect.top;

		const halfHit = OVERLAY_HIT_SCREEN_SIZE / 2;
		const handles: Array<{
			handle: OverlayPointerTarget['handle'];
			x: number;
			y: number;
			cursor: string;
		}> = [
			{ handle: 'nw', x: rect.left, y: rect.top, cursor: 'nwse-resize' },
			{ handle: 'ne', x: rect.right, y: rect.top, cursor: 'nesw-resize' },
			{ handle: 'se', x: rect.right, y: rect.bottom, cursor: 'nwse-resize' },
			{ handle: 'sw', x: rect.left, y: rect.bottom, cursor: 'nesw-resize' }
		];

		for (const handle of handles) {
			if (Math.abs(localX - handle.x) <= halfHit && Math.abs(localY - handle.y) <= halfHit) {
				return {
					handle: handle.handle,
					point,
					cursor: handle.cursor
				};
			}
		}

		const inside =
			localX >= rect.left && localX <= rect.right && localY >= rect.top && localY <= rect.bottom;
		if (inside) {
			return { handle: 'move', point, cursor: 'move' };
		}

		return null;
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

	function drawOverlayControls(rect: NonNullable<ReturnType<typeof getOverlayRect>>) {
		if (!overlayControls || !presentation.overlayMode) return;

		overlayControls
			.rect(rect.left, rect.top, rect.width, rect.height)
			.stroke({ color: 0xffffff, alpha: 0.9, width: OVERLAY_BORDER_SCREEN_WIDTH });

		const handles = [
			[rect.left, rect.top],
			[rect.left + rect.width / 2, rect.top],
			[rect.right, rect.top],
			[rect.right, rect.top + rect.height / 2],
			[rect.right, rect.bottom],
			[rect.left + rect.width / 2, rect.bottom],
			[rect.left, rect.bottom],
			[rect.left, rect.top + rect.height / 2]
		] as const;

		for (const [x, y] of handles) {
			overlayControls
				.circle(x, y, OVERLAY_HANDLE_SCREEN_RADIUS)
				.fill(0xffffff)
				.stroke({ color: 0x000000, alpha: 0.45, width: OVERLAY_BORDER_SCREEN_WIDTH });
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
			!cropOverlay ||
			!overlaySprite ||
			!overlayControls
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
		overlayControls.clear();
		if (presentation.overlay?.enabled && overlayTexture) {
			const rect = getOverlayRect();
			if (rect) {
				overlaySprite.texture = overlayTexture;
				overlaySprite.position.set(rect.centerX, rect.centerY);
				overlaySprite.width = rect.width;
				overlaySprite.height = rect.height;
				overlaySprite.alpha = presentation.overlay.opacity;
				overlaySprite.visible = true;
				drawOverlayControls(rect);
			}
		} else {
			overlaySprite.visible = false;
		}
		requestRender();
	}

	function destroy() {
		sourceRequestId += 1;
		overlayRequestId += 1;
		stopPreviewTransformAnimation();
		cancelScheduledRender();
		resizeObserver?.disconnect();
		void clearTexture();
		void clearOverlayTexture();
		app?.destroy(true, { children: true, texture: true });
		app = null;
		spriteContainer = null;
		rotationContainer = null;
		flipContainer = null;
		sprite = null;
		cropMask = null;
		cropOverlay = null;
		overlaySprite = null;
		overlayControls = null;
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
		getOverlayPointerTarget,
		getOverlayPoint,
		getOverlayHeightRatio() {
			const rect = getOverlayRect();
			if (!rect) return undefined;
			const width = rect.width / rect.frameWidth;
			if (!width) return undefined;
			return rect.height / rect.frameHeight / width;
		},
		zoomPreviewAt,
		zoomPreviewBy,
		panPreviewBy,
		resetPreviewTransform,
		destroy
	};
}

export type PreviewRendererController = ReturnType<typeof createPreviewRenderer>;
