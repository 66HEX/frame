<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import PreviewViewport from './PreviewViewport.svelte';
	import PreviewTimeline from './PreviewTimeline.svelte';
	import {
		AUDIO_ONLY_CONTAINERS,
		type ConversionConfig,
		type CropSettings,
		type MetadataStatus
	} from '$lib/types';
	import {
		createPreviewCrop,
		createPreviewOverlay,
		createPreviewPlayback,
		createPreviewRenderer
	} from '$lib/features/preview';

	type PreviewMediaKind = 'unknown' | 'video' | 'audio' | 'image';

	let {
		filePath,
		mediaKind,
		metadataStatus = 'idle',
		initialStartTime,
		initialEndTime,
		rotation = '0',
		flipHorizontal = false,
		flipVertical = false,
		processingMode = 'reencode',
		container,
		onSave,
		onUpdateConfig,
		controlsDisabled = false,
		initialCrop = null,
		initialOverlay = null,
		sourceWidth,
		sourceHeight
	}: {
		filePath: string;
		mediaKind?: 'video' | 'audio' | 'image';
		metadataStatus?: MetadataStatus;
		initialStartTime?: string;
		initialEndTime?: string;
		rotation?: ConversionConfig['rotation'];
		flipHorizontal?: boolean;
		flipVertical?: boolean;
		processingMode?: ConversionConfig['processingMode'];
		container?: ConversionConfig['container'];
		onSave: (start?: string, end?: string) => void;
		onUpdateConfig?: (config: Partial<ConversionConfig>) => void;
		controlsDisabled?: boolean;
		initialCrop?: CropSettings | null;
		initialOverlay?: ConversionConfig['overlay'];
		sourceWidth?: number;
		sourceHeight?: number;
	} = $props();

	const previewMediaKind = $derived<PreviewMediaKind>(
		metadataStatus === 'ready' && mediaKind ? mediaKind : 'unknown'
	);
	const isImage = $derived(previewMediaKind === 'image');
	const isAudioOnlyOutput = $derived(AUDIO_ONLY_CONTAINERS.includes(container ?? ''));
	const hideVisualControls = $derived(
		previewMediaKind === 'audio' || (previewMediaKind === 'video' && isAudioOnlyOutput)
	);
	const trimDisabled = $derived(controlsDisabled || isImage || previewMediaKind === 'unknown');
	const overlayAvailable = $derived(
		previewMediaKind === 'video' &&
			!isAudioOnlyOutput &&
			processingMode !== 'copy' &&
			(container ?? '').toLowerCase() !== 'gif'
	);

	const playback = createPreviewPlayback({
		isImage: () => previewMediaKind === 'image',
		onSave: (start, end) => onSave(start, end)
	});

	const crop = createPreviewCrop({
		getRotation: () => rotation,
		getFlipHorizontal: () => flipHorizontal,
		getFlipVertical: () => flipVertical,
		getSourceWidth: () => sourceWidth,
		getSourceHeight: () => sourceHeight,
		getControlsDisabled: () => controlsDisabled,
		onUpdateConfig: (config) => onUpdateConfig?.(config)
	});

	const renderer = createPreviewRenderer();

	const overlay = createPreviewOverlay({
		getControlsDisabled: () => controlsDisabled,
		onUpdateConfig: (config) => onUpdateConfig?.(config),
		onDeactivateCrop: () => {
			if (crop.cropMode) {
				crop.toggleCropMode();
			}
		}
	});

	$effect(() => {
		const nextStartTime = initialStartTime;
		const nextEndTime = initialEndTime;
		untrack(() => playback.syncInitialValues(nextStartTime, nextEndTime));
	});

	$effect(() => {
		void initialCrop;
		void rotation;
		void flipHorizontal;
		void flipVertical;
		untrack(() => crop.syncInitialCrop(initialCrop));
	});

	$effect(() => {
		const nextOverlay = initialOverlay;
		untrack(() => overlay.syncInitialOverlay(nextOverlay));
	});

	$effect(() => {
		const naturalWidth = renderer.naturalWidth;
		const naturalHeight = renderer.naturalHeight;
		untrack(() => crop.setNaturalDimensions(naturalWidth, naturalHeight));
	});

	$effect(() => {
		void filePath;
		void previewMediaKind;
		untrack(() => {
			void renderer.setSource(filePath, previewMediaKind);
		});
	});

	$effect(() => {
		const nextMediaKind = previewMediaKind;
		const nextRotation = rotation;
		const nextFlipHorizontal = flipHorizontal;
		const nextFlipVertical = flipVertical;
		const cropMode = crop.cropMode;
		const appliedCrop = crop.appliedCrop;
		const draftCrop = crop.draftCrop;
		const overlayMode = overlay.overlayMode;
		const nextOverlay = overlay.overlay;
		const canUseOverlay = overlayAvailable;
		const nextSourceWidth = sourceWidth;
		const nextSourceHeight = sourceHeight;
		untrack(() =>
			renderer.setPresentationState({
				mediaKind: nextMediaKind,
				rotation: nextRotation,
				flipHorizontal: nextFlipHorizontal,
				flipVertical: nextFlipVertical,
				cropMode,
				appliedCrop,
				draftCrop,
				overlayMode: canUseOverlay && overlayMode,
				overlay: canUseOverlay ? nextOverlay : null,
				sourceWidth: nextSourceWidth,
				sourceHeight: nextSourceHeight
			})
		);
	});

	onMount(() => {
		return () => {
			playback.destroy();
			crop.destroy();
			renderer.destroy();
		};
	});
</script>

<div class="card-highlight flex h-full flex-col rounded-lg bg-frame-gray-100 p-4 shadow-md">
	<PreviewViewport
		{filePath}
		mediaKind={previewMediaKind}
		{renderer}
		{crop}
		{overlay}
		{playback}
		{controlsDisabled}
		{flipHorizontal}
		{flipVertical}
		{overlayAvailable}
		{hideVisualControls}
	/>
	<PreviewTimeline {playback} {trimDisabled} {isImage} />
</div>
