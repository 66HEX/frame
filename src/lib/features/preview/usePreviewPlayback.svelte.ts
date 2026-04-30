function parseTimeToSeconds(timeStr?: string): number {
	if (!timeStr) return 0;
	const parts = timeStr.split(':').map(Number);
	if (parts.length === 3) {
		return parts[0] * 3600 + parts[1] * 60 + parts[2];
	}
	return 0;
}

export function formatTime(seconds: number): string {
	const h = Math.floor(seconds / 3600);
	const m = Math.floor((seconds % 3600) / 60);
	const s = seconds % 60;
	return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toFixed(3).padStart(6, '0')}`;
}

interface PreviewPlaybackOptions {
	isImage: () => boolean;
	onSave: (start?: string, end?: string) => void;
}

export function createPreviewPlayback({ isImage, onSave }: PreviewPlaybackOptions) {
	let mediaElement = $state<HTMLMediaElement | undefined>();
	let sliderElement = $state<HTMLDivElement | undefined>();

	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let startValue = $state(0);
	let endValue = $state(0);
	let dragging = $state<'start' | 'end' | 'scrub' | null>(null);
	let wasPlayingBeforeScrub = false;
	let previousInitialStart: string | undefined;
	let previousInitialEnd: string | undefined;

	function detachMediaListeners(element?: HTMLMediaElement) {
		if (!element) return;
		element.removeEventListener('loadedmetadata', syncFromMedia);
		element.removeEventListener('durationchange', syncFromMedia);
		element.removeEventListener('timeupdate', handleTimeUpdate);
		element.removeEventListener('play', handlePlay);
		element.removeEventListener('pause', handlePause);
	}

	function attachMediaListeners(element?: HTMLMediaElement) {
		if (!element) return;
		element.addEventListener('loadedmetadata', syncFromMedia);
		element.addEventListener('durationchange', syncFromMedia);
		element.addEventListener('timeupdate', handleTimeUpdate);
		element.addEventListener('play', handlePlay);
		element.addEventListener('pause', handlePause);
	}

	function setMediaElement(element?: HTMLMediaElement) {
		if (mediaElement === element) return;
		detachMediaListeners(mediaElement);
		mediaElement = element;
		attachMediaListeners(mediaElement);

		if (!mediaElement) {
			isPlaying = false;
			currentTime = 0;
			duration = 0;
			startValue = 0;
			endValue = 0;
			return;
		}

		isPlaying = !mediaElement.paused;
		currentTime = mediaElement.currentTime || 0;
		syncFromMedia();
	}

	function syncInitialValues(initialStartTime?: string, initialEndTime?: string) {
		if (initialStartTime !== previousInitialStart) {
			previousInitialStart = initialStartTime;
			startValue = initialStartTime ? parseTimeToSeconds(initialStartTime) : 0;
		}

		if (initialEndTime !== previousInitialEnd) {
			previousInitialEnd = initialEndTime;
			if (initialEndTime) {
				endValue = parseTimeToSeconds(initialEndTime);
			} else if (duration) {
				endValue = duration;
			}
		} else if (!initialEndTime && duration && endValue === 0) {
			endValue = duration;
		}
	}

	function syncFromMedia() {
		if (!mediaElement || isImage()) return;
		const nextDuration = Number.isFinite(mediaElement.duration) ? mediaElement.duration : 0;
		duration = nextDuration;
		currentTime = mediaElement.currentTime || 0;

		if (previousInitialEnd) {
			endValue = parseTimeToSeconds(previousInitialEnd);
		} else {
			endValue = duration;
		}

		if (startValue > duration) startValue = 0;
		if (endValue > duration) endValue = duration;
	}

	function handlePlay() {
		isPlaying = true;
	}

	function handlePause() {
		isPlaying = false;
	}

	function handleTimeUpdate() {
		if (isImage() || !mediaElement) return;
		currentTime = mediaElement.currentTime;
		if (dragging) return;

		if (currentTime >= endValue && endValue > startValue) {
			mediaElement.pause();
			isPlaying = false;
			mediaElement.currentTime = startValue;
			currentTime = startValue;
		}
	}

	function togglePlay() {
		if (isImage() || !mediaElement) return;
		if (isPlaying) {
			mediaElement.pause();
			return;
		}

		if (mediaElement.currentTime < startValue || mediaElement.currentTime >= endValue) {
			mediaElement.currentTime = startValue;
			currentTime = startValue;
		}

		void mediaElement.play();
	}

	function commitTrimValues() {
		if (isImage()) return;
		const startStr = startValue > 0 ? formatTime(startValue) : undefined;
		const endStr = duration > 0 && endValue < duration ? formatTime(endValue) : undefined;
		onSave(startStr, endStr);
	}

	function setSliderElement(element?: HTMLDivElement) {
		sliderElement = element;
	}

	function beginHandleDrag(event: MouseEvent, type: 'start' | 'end') {
		if (isImage()) return;
		event.preventDefault();
		event.stopPropagation();
		dragging = type;
		window.addEventListener('mousemove', handleMouseMove);
		window.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(event: MouseEvent) {
		if (isImage() || !dragging || !sliderElement) return;

		const rect = sliderElement.getBoundingClientRect();
		const percent = Math.min(Math.max((event.clientX - rect.left) / rect.width, 0), 1);
		const time = percent * duration;

		if (dragging === 'scrub') {
			currentTime = time;
			if (mediaElement) mediaElement.currentTime = currentTime;
			return;
		}

		if (dragging === 'start') {
			startValue = Math.min(time, endValue - 1);
			if (mediaElement) mediaElement.currentTime = startValue;
		} else if (dragging === 'end') {
			endValue = Math.max(time, startValue + 1);
			if (mediaElement) mediaElement.currentTime = endValue;
		}

		commitTrimValues();
	}

	function handleMouseUp() {
		if (isImage()) return;
		if (dragging === 'scrub') {
			if (wasPlayingBeforeScrub && mediaElement) {
				void mediaElement.play();
			}
		} else if (dragging) {
			commitTrimValues();
		}

		dragging = null;
		window.removeEventListener('mousemove', handleMouseMove);
		window.removeEventListener('mouseup', handleMouseUp);
	}

	function seekTo(event: MouseEvent) {
		if (isImage() || !sliderElement) return;

		const rect = sliderElement.getBoundingClientRect();
		const percent = Math.min(Math.max((event.clientX - rect.left) / rect.width, 0), 1);
		const time = percent * duration;

		if (mediaElement) {
			mediaElement.currentTime = time;
			currentTime = time;
		}

		dragging = 'scrub';
		wasPlayingBeforeScrub = isPlaying;
		if (isPlaying && mediaElement) {
			mediaElement.pause();
		}

		window.addEventListener('mousemove', handleMouseMove);
		window.addEventListener('mouseup', handleMouseUp);
	}

	function toTimelinePercent(value: number): number {
		if (!duration || !Number.isFinite(duration) || duration <= 0) return 0;
		return (value / duration) * 100;
	}

	function destroy() {
		window.removeEventListener('mousemove', handleMouseMove);
		window.removeEventListener('mouseup', handleMouseUp);
		detachMediaListeners(mediaElement);
	}

	return {
		get mediaElement() {
			return mediaElement;
		},
		get isPlaying() {
			return isPlaying;
		},
		get currentTime() {
			return currentTime;
		},
		get duration() {
			return duration;
		},
		get startValue() {
			return startValue;
		},
		get endValue() {
			return endValue;
		},
		get dragging() {
			return dragging;
		},
		setMediaElement,
		setSliderElement,
		syncInitialValues,
		togglePlay,
		commitTrimValues,
		beginHandleDrag,
		seekTo,
		toTimelinePercent,
		destroy,
		setStartValue(value: number) {
			startValue = value;
		},
		setEndValue(value: number) {
			endValue = value;
		}
	};
}

export type PreviewPlaybackController = ReturnType<typeof createPreviewPlayback>;
