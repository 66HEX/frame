import {
	Color,
	Geometry,
	GlProgram,
	Mesh,
	RenderTexture,
	Shader,
	UniformGroup,
	WebGLRenderer
} from 'pixi.js';
import {
	blurFragment,
	compositeFragment,
	sourceFragment,
	vertex
} from './hero-shaders';

// Keep the presentation pass at device resolution. Only bloom may be downsampled.
// 8M presentation pixels + three <=1.125M bloom targets use <=46 MiB of RGBA8 storage.
const MAX_PIXELS = 8_000_000;
const MAX_DPR = 4;
const MAX_BLOOM_PIXELS = 1_125_000;
const FRAME_INTERVAL = 1000 / 40;

export function renderSize(
	width: number,
	height: number,
	dpr: number,
	maxDimension = 4096
) {
	const ratio = Math.min(
		MAX_DPR,
		dpr,
		Math.sqrt(MAX_PIXELS / (width * height)),
		maxDimension / width,
		maxDimension / height
	);
	return {
		width: Math.max(1, Math.floor(width * ratio)),
		height: Math.max(1, Math.floor(height * ratio)),
		ratio
	};
}

function cleanupAll(disposers: Array<() => void>) {
	for (const dispose of disposers.splice(0).reverse()) {
		try {
			dispose();
		} catch {
			/* A failed disposer must not retain the remaining resources. */
		}
	}
}

async function createScene(
	host: HTMLElement,
	signal: AbortSignal,
	onLoss: () => void
) {
	const cleanup: Array<() => void> = [];
	try {
		const renderer = new WebGLRenderer();
		cleanup.push(() => renderer.destroy(false));
		const canvas = document.createElement('canvas');
		canvas.className = 'absolute left-0 block w-full';
		canvas.setAttribute('aria-hidden', 'true');
		cleanup.push(() => canvas.remove());
		await renderer.init({
			canvas,
			width: 1,
			height: 1,
			resolution: 1,
			backgroundAlpha: 0,
			antialias: false,
			autoDensity: false,
			powerPreference: 'low-power',
			preferWebGLVersion: 2,
			skipExtensionImports: true
		});
		if (signal.aborted) throw new DOMException('Unmounted', 'AbortError');
		if (renderer.context.webGLVersion !== 2)
			throw new Error('The hero requires WebGL 2.');
		const lost = (event: Event) => {
			event.preventDefault();
			onLoss();
		};
		canvas.addEventListener('webglcontextlost', lost);
		cleanup.push(() => canvas.removeEventListener('webglcontextlost', lost));
		const maxDimension = Math.min(
			renderer.gl.getParameter(renderer.gl.MAX_TEXTURE_SIZE),
			renderer.gl.getParameter(renderer.gl.MAX_RENDERBUFFER_SIZE),
			4096
		);
		const geometry = new Geometry({
			attributes: {
				aPosition: {
					buffer: new Float32Array([0, 0, 1, 0, 1, 1, 0, 1]),
					format: 'float32x2'
				},
				aUV: {
					buffer: new Float32Array([0, 0, 1, 0, 1, 1, 0, 1]),
					format: 'float32x2'
				}
			},
			indexBuffer: [0, 1, 2, 0, 2, 3]
		});
		cleanup.push(() => geometry.destroy(true));
		const uniforms = new UniformGroup({
			uSize: { value: new Float32Array([1, 1]), type: 'vec2<f32>' },
			uView: { value: new Float32Array([0, 1]), type: 'vec2<f32>' },
			uPointer: { value: new Float32Array([-10, -10]), type: 'vec2<f32>' },
			uTime: { value: 0, type: 'f32' },
			uPixelRatio: { value: 1, type: 'f32' },
			uBloomPixelRatio: { value: 0.5, type: 'f32' },
			uDark: { value: 1, type: 'f32' },
			uPrimary: { value: new Float32Array(3), type: 'vec3<f32>' },
			uAccent: { value: new Float32Array(3), type: 'vec3<f32>' },
			uSecondary: { value: new Float32Array(3), type: 'vec3<f32>' }
		});
		function target() {
			const texture = RenderTexture.create({
				width: 1,
				height: 1,
				resolution: 1,
				format: 'rgba8unorm',
				scaleMode: 'linear',
				addressMode: 'clamp-to-edge',
				alphaMode: 'premultiplied-alpha',
				autoGenerateMipmaps: false
			});
			cleanup.push(() => texture.destroy(true));
			return texture;
		}
		const source = target();
		const horizontal = target();
		const bloom = target();
		function pass(fragment: string, resources: Record<string, unknown>) {
			// Programs belong to this scene, not Pixi's shared program cache.
			const program = new GlProgram({ vertex, fragment });
			cleanup.push(() => program.destroy());
			const shader = new Shader({ glProgram: program, resources });
			cleanup.push(() => shader.destroy());
			const mesh = new Mesh({ geometry, shader });
			mesh.blendMode = 'none';
			mesh.eventMode = 'none';
			cleanup.push(() => mesh.destroy());
			return mesh;
		}
		const directionX = new UniformGroup({
			uDirection: { value: new Float32Array([1, 0]), type: 'vec2<f32>' }
		});
		const directionY = new UniformGroup({
			uDirection: { value: new Float32Array([0, 1]), type: 'vec2<f32>' }
		});
		const sourceMesh = pass(sourceFragment, { field: uniforms });
		const horizontalMesh = pass(blurFragment, {
			blur: directionX,
			uSource: source.source
		});
		const verticalMesh = pass(blurFragment, {
			blur: directionY,
			uSource: horizontal.source
		});
		const compositeMesh = pass(compositeFragment, {
			field: uniforms,
			uBloom: bloom.source
		});
		let physicalWidth = 0;
		let physicalHeight = 0;
		let logicalWidth = 0;
		let logicalHeight = 0;
		function updateViewport() {
			if (!logicalWidth || !logicalHeight) return;
			// Rasterize only the viewport plus a 64px margin. Zoom can then keep native
			// pixel density without spending the presentation budget below the fold.
			const viewHeight = Math.min(logicalHeight, window.innerHeight + 128);
			const viewTop = Math.max(
				0,
				Math.min(
					Math.floor((-host.getBoundingClientRect().top - 64) / 32) * 32,
					logicalHeight - viewHeight
				)
			);
			const size = renderSize(
				logicalWidth,
				viewHeight,
				window.devicePixelRatio || 1,
				maxDimension
			);
			uniforms.uniforms.uView.set([
				viewTop / logicalHeight,
				viewHeight / logicalHeight
			]);
			uniforms.uniforms.uPixelRatio = size.ratio;
			canvas.style.top = `${viewTop}px`;
			canvas.style.height = `${viewHeight}px`;
			if (physicalWidth !== size.width || physicalHeight !== size.height) {
				physicalWidth = size.width;
				physicalHeight = size.height;
				renderer.resize(size.width, size.height, 1);
				compositeMesh.scale.set(size.width, size.height);
			}
			const bloomRatio = Math.min(
				size.ratio * 0.5,
				0.75,
				Math.sqrt(MAX_BLOOM_PIXELS / (logicalWidth * viewHeight))
			);
			const halfWidth = Math.max(1, Math.floor(logicalWidth * bloomRatio));
			const halfHeight = Math.max(1, Math.floor(viewHeight * bloomRatio));
			uniforms.uniforms.uBloomPixelRatio = bloomRatio;
			if (source.width === halfWidth && source.height === halfHeight) return;
			for (const texture of [source, horizontal, bloom])
				texture.resize(halfWidth, halfHeight);
			for (const mesh of [sourceMesh, horizontalMesh, verticalMesh])
				mesh.scale.set(halfWidth, halfHeight);
			directionX.uniforms.uDirection.set([1 / halfWidth, 0]);
			directionY.uniforms.uDirection.set([0, 1 / halfHeight]);
		}
		function resize(width: number, height: number) {
			logicalWidth = width;
			logicalHeight = height;
			uniforms.uniforms.uSize.set([width, height]);
			updateViewport();
		}
		function theme() {
			const style = getComputedStyle(host);
			for (const [name, uniform] of [
				['primary', 'uPrimary'],
				['accent', 'uAccent'],
				['secondary', 'uSecondary']
			] as const) {
				const srgb = new Color(
					style.getPropertyValue(`--hero-shader-${name}`).trim()
				).toRgbArray();
				uniforms.uniforms[uniform].set(
					srgb.map((value) =>
						value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
					)
				);
			}
			uniforms.uniforms.uDark = Number(
				document.documentElement.classList.contains('dark')
			);
		}
		function draw(time: number, x: number, y: number) {
			uniforms.uniforms.uTime = time;
			uniforms.uniforms.uPointer.set([x, y]);
			renderer.render({ container: sourceMesh, target: source, clear: true });
			renderer.render({
				container: horizontalMesh,
				target: horizontal,
				clear: true
			});
			renderer.render({ container: verticalMesh, target: bloom, clear: true });
			renderer.render({ container: compositeMesh, clear: true });
			if (renderer.gl.isContextLost()) throw new Error('Hero context lost.');
		}
		host.append(canvas);
		return {
			resize,
			updateViewport,
			theme,
			draw,
			destroy: () => cleanupAll(cleanup)
		};
	} catch (error) {
		cleanupAll(cleanup);
		throw error;
	}
}

export interface HeroController {
	destroy(): void;
}

/** Owns every canvas, pass, observer and frame for the decorative hero shader. */
export function mountHeroShader(
	host: HTMLElement,
	onState: (ready: boolean) => void
): HeroController {
	const abort = new AbortController();
	const cleanup: Array<() => void> = [];
	const motion = matchMedia('(prefers-reduced-motion: reduce)');
	const forcedColors = matchMedia('(forced-colors: active)');
	const parent = host.parentElement!;
	let scene: Awaited<ReturnType<typeof createScene>> | undefined;
	let frame = 0;
	let retry = 0;
	let attempts = 0;
	let generation = 0;
	let visible = false;
	let dirty = true;
	let ready = false;
	let width = 0;
	let height = 0;
	let last = 0;
	let time = 0;
	let pointerX = -10;
	let pointerY = -10;
	let currentX = -10;
	let currentY = -10;
	const canDraw = () =>
		!abort.signal.aborted &&
		!!scene &&
		visible &&
		!document.hidden &&
		!forcedColors.matches &&
		width > 0 &&
		height > 0;
	const animated = () => !motion.matches;
	function status(value: boolean) {
		ready = value;
		onState(ready);
	}
	function stop() {
		cancelAnimationFrame(frame);
		frame = 0;
		last = 0;
	}
	function schedule() {
		if (!canDraw()) {
			stop();
			return;
		}
		if (!frame && (dirty || animated())) frame = requestAnimationFrame(tick);
	}
	function invalidate() {
		dirty = true;
		schedule();
	}
	function fail(error?: unknown) {
		generation++;
		clearTimeout(retry);
		stop();
		status(false);
		scene?.destroy();
		scene = undefined;
		if (import.meta.env.DEV && error)
			console.warn('Hero shader rendering failed.', error);
		if (!abort.signal.aborted && attempts++ < 2)
			retry = window.setTimeout(() => {
				void initialize();
			}, 250 * attempts);
	}
	function tick(now: number) {
		frame = 0;
		if (!canDraw()) {
			last = 0;
			return;
		}
		if (!dirty && last && now - last < FRAME_INTERVAL) {
			schedule();
			return;
		}
		const delta = last ? Math.min((now - last) / 1000, 0.1) : 0;
		last = now;
		if (animated()) {
			time += delta;
			const damping = 1 - Math.exp(-delta * 4);
			currentX += (pointerX - currentX) * damping;
			currentY += (pointerY - currentY) * damping;
		}
		try {
			scene!.draw(
				time,
				animated() ? currentX : -10,
				animated() ? currentY : -10
			);
			dirty = false;
			if (!ready) status(true);
		} catch (error) {
			fail(error);
			return;
		}
		schedule();
	}
	async function initialize() {
		const id = ++generation;
		try {
			const created = await createScene(host, abort.signal, () => {
				// Dispose outside the context event dispatch, after Pixi's own handler.
				queueMicrotask(() => {
					if (id === generation && !abort.signal.aborted) fail();
				});
			});
			if (abort.signal.aborted || id !== generation) {
				created.destroy();
				return;
			}
			scene = created;
			scene.theme();
			if (width > 0 && height > 0) scene.resize(width, height);
			invalidate();
		} catch (error) {
			if (!abort.signal.aborted && id === generation) fail(error);
		}
	}
	const resize = new ResizeObserver(([entry]) => {
		width = entry.contentRect.width;
		height = entry.contentRect.height;
		try {
			if (width > 0 && height > 0) scene?.resize(width, height);
		} catch (error) {
			fail(error);
		}
		invalidate();
	});
	resize.observe(host);
	cleanup.push(() => resize.disconnect());
	const intersection = new IntersectionObserver(([entry]) => {
		visible = entry.isIntersecting;
		invalidate();
	});
	intersection.observe(host);
	cleanup.push(() => intersection.disconnect());
	function viewportChanged() {
		try {
			scene?.updateViewport();
		} catch (error) {
			fail(error);
		}
		invalidate();
	}
	window.addEventListener('scroll', viewportChanged, { passive: true });
	window.addEventListener('resize', viewportChanged, { passive: true });
	cleanup.push(() => {
		window.removeEventListener('scroll', viewportChanged);
		window.removeEventListener('resize', viewportChanged);
	});
	const theme = new MutationObserver(() => {
		try {
			scene?.theme();
		} catch (error) {
			fail(error);
		}
		invalidate();
	});
	theme.observe(document.documentElement, {
		attributes: true,
		attributeFilter: ['class']
	});
	cleanup.push(() => theme.disconnect());
	function preferenceChanged() {
		stop();
		status(ready);
		invalidate();
	}
	motion.addEventListener('change', preferenceChanged);
	forcedColors.addEventListener('change', preferenceChanged);
	document.addEventListener('visibilitychange', invalidate);
	cleanup.push(() => {
		motion.removeEventListener('change', preferenceChanged);
		forcedColors.removeEventListener('change', preferenceChanged);
		document.removeEventListener('visibilitychange', invalidate);
	});
	function pointerMove(event: PointerEvent) {
		if (event.pointerType !== 'mouse' || !animated()) return;
		const rect = host.getBoundingClientRect();
		pointerX = (event.clientX - rect.left) / rect.width;
		pointerY = (event.clientY - rect.top) / rect.height;
	}
	function pointerLeave() {
		pointerX = -10;
		pointerY = -10;
	}
	parent.addEventListener('pointermove', pointerMove, { passive: true });
	parent.addEventListener('pointerleave', pointerLeave);
	cleanup.push(() => {
		parent.removeEventListener('pointermove', pointerMove);
		parent.removeEventListener('pointerleave', pointerLeave);
	});
	// A DPR change may occur without a CSS resize when moving between displays.
	let dprQuery: MediaQueryList;
	function dprChanged() {
		dprQuery?.removeEventListener('change', dprChanged);
		dprQuery = matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
		dprQuery.addEventListener('change', dprChanged);
		try {
			if (width > 0 && height > 0) scene?.resize(width, height);
		} catch (error) {
			fail(error);
		}
		invalidate();
	}
	dprChanged();
	cleanup.push(() => dprQuery.removeEventListener('change', dprChanged));
	status(false);
	void initialize();
	return {
		destroy() {
			if (abort.signal.aborted) return;
			abort.abort();
			generation++;
			clearTimeout(retry);
			stop();
			cleanupAll(cleanup);
			scene?.destroy();
			scene = undefined;
		}
	};
}
