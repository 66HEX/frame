import { describe, expect, test } from 'bun:test';
import {
	type CropRect,
	applyVisualCropDrag,
	adjustRectToRatio,
	clampRect,
	enforceAspect,
	remapDragDeltas,
	transformCropRect
} from './crop';

function expectRectClose(actual: CropRect, expected: CropRect, precision = 6) {
	expect(actual.x).toBeCloseTo(expected.x, precision);
	expect(actual.y).toBeCloseTo(expected.y, precision);
	expect(actual.width).toBeCloseTo(expected.width, precision);
	expect(actual.height).toBeCloseTo(expected.height, precision);
}

describe('crop utilities', () => {
	test('transformCropRect round-trips rotations and flips', () => {
		const rect: CropRect = { x: 0.2, y: 0.15, width: 0.35, height: 0.45 };

		for (const rotation of ['0', '90', '180', '270']) {
			for (const flipHorizontal of [false, true]) {
				for (const flipVertical of [false, true]) {
					const transformed = transformCropRect(
						rect,
						rotation,
						flipHorizontal,
						flipVertical,
						false
					);
					const roundTrip = transformCropRect(
						transformed,
						rotation,
						flipHorizontal,
						flipVertical,
						true
					);

					expectRectClose(roundTrip, rect);
				}
			}
		}
	});

	test('remapDragDeltas follows rotation and flip orientation', () => {
		expect(remapDragDeltas(0.2, 0.1, '0', false, false)).toEqual({ dx: 0.2, dy: 0.1 });
		expect(remapDragDeltas(0.2, 0.1, '90', false, false)).toEqual({ dx: 0.1, dy: -0.2 });
		expect(remapDragDeltas(0.2, 0.1, '180', false, false)).toEqual({ dx: -0.2, dy: -0.1 });
		expect(remapDragDeltas(0.2, 0.1, '270', false, false)).toEqual({ dx: -0.1, dy: 0.2 });
		expect(remapDragDeltas(0.2, 0.1, '90', true, true)).toEqual({ dx: -0.1, dy: 0.2 });
	});

	test('clampRect keeps crop inside normalized bounds', () => {
		expectRectClose(clampRect({ x: -0.2, y: 0.9, width: 0.2, height: 0.3 }), {
			x: 0,
			y: 0.7,
			width: 0.2,
			height: 0.3
		});
		expectRectClose(clampRect({ x: 0.9, y: 0.9, width: 0.01, height: 0.01 }), {
			x: 0.9,
			y: 0.9,
			width: 0.05,
			height: 0.05
		});
	});

	test('adjustRectToRatio preserves center when possible', () => {
		const adjusted = adjustRectToRatio(
			{ x: 0.2, y: 0.2, width: 0.6, height: 0.4 },
			1,
			1920,
			1080,
			false
		);

		expect(adjusted.x + adjusted.width / 2).toBeCloseTo(0.5, 6);
		expect(adjusted.y + adjusted.height / 2).toBeCloseTo(0.4, 6);
		expect(adjusted.width / adjusted.height).toBeCloseTo(1 / (1920 / 1080), 6);
	});

	test('enforceAspect anchors the dragged edge', () => {
		const startRect: CropRect = { x: 0.2, y: 0.2, width: 0.4, height: 0.4 };
		const next = enforceAspect(
			{ x: 0.2, y: 0.2, width: 0.55, height: 0.4 },
			'e',
			startRect,
			16 / 9,
			1920,
			1080,
			false
		);

		expect(next.x).toBe(startRect.x);
		expect(next.y + next.height / 2).toBeCloseTo(startRect.y + startRect.height / 2, 6);
		expect(next.width / next.height).toBeCloseTo(1, 6);
	});

	test('applyVisualCropDrag keeps drag deltas in visual space for side rotations', () => {
		const startRect: CropRect = { x: 0.2, y: 0.2, width: 0.4, height: 0.4 };
		const next = applyVisualCropDrag({
			startRect,
			handle: 'e',
			startPoint: { x: 0.6, y: 0.4 },
			currentPoint: { x: 0.7, y: 0.4 },
			aspectId: 'free',
			sourceWidth: 1920,
			sourceHeight: 1080,
			isSideRotation: true
		});

		expectRectClose(next, { x: 0.2, y: 0.2, width: 0.5, height: 0.4 });
	});

	test('applyVisualCropDrag moves crop directly in visual space for side rotations', () => {
		const startRect: CropRect = { x: 0.2, y: 0.2, width: 0.4, height: 0.4 };
		const next = applyVisualCropDrag({
			startRect,
			handle: 'move',
			startPoint: { x: 0.4, y: 0.4 },
			currentPoint: { x: 0.5, y: 0.45 },
			aspectId: 'free',
			sourceWidth: 1920,
			sourceHeight: 1080,
			isSideRotation: true
		});

		expectRectClose(next, { x: 0.3, y: 0.25, width: 0.4, height: 0.4 });
	});
});
