export type Rect = Pick<DOMRect, 'left' | 'top' | 'width' | 'height'>;
type Corner = [number, number];

export type Surface = {
	rect: Rect;
	corners: Corner[];
	shadow: string;
};

// Computed box shadows use px lengths. Scale offsets per axis and the blur/spread
// uniformly; our media keeps its aspect ratio throughout the transition.
export function scaleShadow(shadow: string, x: number, y: number) {
	return shadow
		.split(/,(?![^(]*\))/)
		.map((layer) => {
			let length = 0;
			return layer.replace(/(-?[\d.]+)px/g, (_, value: string) => {
				const index = length++;
				const scale = index === 0 ? x : index === 1 ? y : Math.sqrt(x * y);
				return `${Number(value) * scale}px`;
			});
		})
		.join(',');
}

export function readSurface(element: HTMLElement, transformed = false): Surface {
	const rect = element.getBoundingClientRect();
	const style = getComputedStyle(element);
	const width = Number.parseFloat(style.width);
	const height = Number.parseFloat(style.height);
	const x = transformed && width ? rect.width / width : 1;
	const y = transformed && height ? rect.height / height : 1;
	const corners = [
		style.borderTopLeftRadius,
		style.borderTopRightRadius,
		style.borderBottomRightRadius,
		style.borderBottomLeftRadius
	].map((corner): Corner => {
		const [rx, ry = rx] = corner.split(' ').map(Number.parseFloat);
		return [rx * x, ry * y];
	});
	return { rect, corners, shadow: scaleShadow(style.boxShadow, x, y) };
}

export function radius(corners: Corner[], x = 1, y = 1) {
	return `${corners.map(([rx]) => `${rx / x}px`).join(' ')} / ${corners.map(([, ry]) => `${ry / y}px`).join(' ')}`;
}

export function flipKeyframes(from: Surface, to: Surface): Keyframe[] {
	// Sampling the inverse scale keeps the visible radius continuous, including
	// when an in-flight transition becomes the starting surface of a reversal.
	return Array.from({ length: 31 }, (_, index) => {
		const progress = index / 30;
		const remaining = 1 - progress;
		const x = 1 + (from.rect.width / to.rect.width - 1) * remaining;
		const y = 1 + (from.rect.height / to.rect.height - 1) * remaining;
		const corners = from.corners.map(([rx, ry], i): Corner => [
			rx * remaining + to.corners[i][0] * progress,
			ry * remaining + to.corners[i][1] * progress
		]);
		return {
			offset: progress,
			transform: `translate(${(from.rect.left - to.rect.left) * remaining}px, ${(from.rect.top - to.rect.top) * remaining}px) scale(${x}, ${y})`,
			borderRadius: radius(corners, x, y)
		};
	});
}
