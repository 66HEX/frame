/** Decode is an animation enhancement, never an unbounded gate on interaction. */
export function decodeImage(
	image: HTMLImageElement,
	signal: AbortSignal,
	budget = 200
): Promise<boolean> {
	return new Promise((resolve) => {
		if (signal.aborted) return resolve(false);
		const finish = (decoded = false) => {
			clearTimeout(timeout);
			signal.removeEventListener('abort', abort);
			resolve(decoded);
		};
		const abort = () => finish();
		const timeout = setTimeout(abort, budget);
		signal.addEventListener('abort', abort, { once: true });
		try {
			image.decode().then(() => finish(image.naturalWidth > 0), abort);
		} catch {
			finish();
		}
	});
}
