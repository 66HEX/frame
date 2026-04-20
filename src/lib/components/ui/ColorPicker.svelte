<script lang="ts">
	import { tick } from 'svelte';
	import { cubicInOut } from 'svelte/easing';
	import { scale } from 'svelte/transition';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import { IconChevronsUpDown } from '$lib/icons';

	const PANEL_WIDTH = 220;
	const PANEL_OFFSET = 4;
	const VIEWPORT_PADDING = 8;
	const FALLBACK_PANEL_HEIGHT = 190;

	let {
		value = $bindable('#ffffff'),
		disabled = false,
		id,
		onchange
	}: {
		value?: string;
		disabled?: boolean;
		id?: string;
		onchange?: (value: string) => void;
	} = $props();

	let open = $state(false);
	let draggingSv = $state(false);
	let draggingHue = $state(false);

	let triggerRef: HTMLButtonElement | undefined = $state();
	let panelRef: HTMLDivElement | undefined = $state();
	let svRef: HTMLButtonElement | undefined = $state();
	let hueRef: HTMLButtonElement | undefined = $state();
	let dropdownStyle = $state('');

	const normalizedValue = $derived(normalizeHex(value ?? '#ffffff'));
	const hsv = $derived(hexToHsv(normalizedValue));
	let hexDraft = $derived(normalizedValue.toUpperCase());

	function clamp(value: number, min: number, max: number) {
		return Math.min(max, Math.max(min, value));
	}

	function normalizeHex(raw: string, fallback = '#ffffff') {
		const fallbackHex = fallback.toLowerCase();
		if (!raw) return fallbackHex;

		let source = raw.trim();
		if (!source.startsWith('#')) source = `#${source}`;

		const shortMatch = source.match(/^#([0-9a-fA-F]{3})$/);
		if (shortMatch) {
			const [r, g, b] = shortMatch[1].split('');
			return `#${r}${r}${g}${g}${b}${b}`.toLowerCase();
		}

		const longMatch = source.match(/^#([0-9a-fA-F]{6})$/);
		if (longMatch) {
			return `#${longMatch[1]}`.toLowerCase();
		}

		return fallbackHex;
	}

	function hexToRgb(hex: string) {
		const raw = normalizeHex(hex).slice(1);
		return {
			r: Number.parseInt(raw.slice(0, 2), 16),
			g: Number.parseInt(raw.slice(2, 4), 16),
			b: Number.parseInt(raw.slice(4, 6), 16)
		};
	}

	function rgbToHex(r: number, g: number, b: number) {
		const toByte = (channel: number) => clamp(Math.round(channel), 0, 255);
		return `#${toByte(r).toString(16).padStart(2, '0')}${toByte(g).toString(16).padStart(2, '0')}${toByte(
			b
		)
			.toString(16)
			.padStart(2, '0')}`;
	}

	function hexToHsv(hex: string) {
		const { r, g, b } = hexToRgb(hex);
		const rNorm = r / 255;
		const gNorm = g / 255;
		const bNorm = b / 255;

		const max = Math.max(rNorm, gNorm, bNorm);
		const min = Math.min(rNorm, gNorm, bNorm);
		const delta = max - min;

		let h = 0;
		if (delta !== 0) {
			if (max === rNorm) {
				h = ((gNorm - bNorm) / delta) % 6;
			} else if (max === gNorm) {
				h = (bNorm - rNorm) / delta + 2;
			} else {
				h = (rNorm - gNorm) / delta + 4;
			}
			h *= 60;
			if (h < 0) h += 360;
		}

		const s = max === 0 ? 0 : delta / max;
		const v = max;
		return { h, s, v };
	}

	function hsvToHex(h: number, s: number, v: number) {
		const hue = ((h % 360) + 360) % 360;
		const sat = clamp(s, 0, 1);
		const val = clamp(v, 0, 1);
		const chroma = val * sat;
		const x = chroma * (1 - Math.abs(((hue / 60) % 2) - 1));
		const m = val - chroma;

		let rPrime = 0;
		let gPrime = 0;
		let bPrime = 0;

		if (hue < 60) {
			rPrime = chroma;
			gPrime = x;
		} else if (hue < 120) {
			rPrime = x;
			gPrime = chroma;
		} else if (hue < 180) {
			gPrime = chroma;
			bPrime = x;
		} else if (hue < 240) {
			gPrime = x;
			bPrime = chroma;
		} else if (hue < 300) {
			rPrime = x;
			bPrime = chroma;
		} else {
			rPrime = chroma;
			bPrime = x;
		}

		return rgbToHex((rPrime + m) * 255, (gPrime + m) * 255, (bPrime + m) * 255);
	}

	function commitHex(next: string) {
		const normalized = normalizeHex(next, normalizedValue);
		value = normalized;
		onchange?.(normalized);
		hexDraft = normalized.toUpperCase();
	}

	function updatePosition() {
		if (!triggerRef) return;
		const rect = triggerRef.getBoundingClientRect();
		const panelHeight = panelRef?.getBoundingClientRect().height ?? FALLBACK_PANEL_HEIGHT;

		const maxLeft = Math.max(VIEWPORT_PADDING, window.innerWidth - PANEL_WIDTH - VIEWPORT_PADDING);
		const left = clamp(rect.left, VIEWPORT_PADDING, maxLeft);

		const belowTop = rect.bottom + PANEL_OFFSET;
		const aboveTop = rect.top - PANEL_OFFSET - panelHeight;
		let top = belowTop;
		if (
			belowTop + panelHeight > window.innerHeight - VIEWPORT_PADDING &&
			aboveTop >= VIEWPORT_PADDING
		) {
			top = aboveTop;
		}

		const maxTop = Math.max(VIEWPORT_PADDING, window.innerHeight - panelHeight - VIEWPORT_PADDING);
		top = clamp(top, VIEWPORT_PADDING, maxTop);

		dropdownStyle = `position:fixed;top:${top}px;left:${left}px;width:${PANEL_WIDTH}px`;
	}

	async function openPicker() {
		if (disabled) return;
		hexDraft = normalizedValue.toUpperCase();
		open = true;
		await tick();
		updatePosition();
	}

	function closePicker() {
		open = false;
		draggingSv = false;
		draggingHue = false;
		hexDraft = normalizedValue.toUpperCase();
	}

	function togglePicker() {
		if (open) {
			closePicker();
			return;
		}
		openPicker();
	}

	function updateFromSv(clientX: number, clientY: number) {
		if (!svRef) return;
		const rect = svRef.getBoundingClientRect();
		if (rect.width <= 0 || rect.height <= 0) return;
		const x = clamp((clientX - rect.left) / rect.width, 0, 1);
		const y = clamp((clientY - rect.top) / rect.height, 0, 1);
		commitHex(hsvToHex(hsv.h, x, 1 - y));
	}

	function updateFromHue(clientX: number) {
		if (!hueRef) return;
		const rect = hueRef.getBoundingClientRect();
		if (rect.width <= 0) return;
		const x = clamp((clientX - rect.left) / rect.width, 0, 1);
		commitHex(hsvToHex(x * 360, hsv.s, hsv.v));
	}

	function startSvDrag(e: MouseEvent) {
		e.preventDefault();
		draggingSv = true;
		updateFromSv(e.clientX, e.clientY);
	}

	function startHueDrag(e: MouseEvent) {
		e.preventDefault();
		draggingHue = true;
		updateFromHue(e.clientX);
	}

	function isCompleteHex(raw: string) {
		return /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(raw);
	}

	function onHexInput(e: Event) {
		const raw = (e.currentTarget as HTMLInputElement).value.trim().toUpperCase();
		const next = raw.startsWith('#') ? raw : `#${raw}`;
		hexDraft = next;
		if (isCompleteHex(next)) {
			commitHex(next);
		}
	}

	function onHexBlur() {
		commitHex(hexDraft);
	}

	function onHexKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			commitHex(hexDraft);
			return;
		}

		if (e.key === 'Escape') {
			e.preventDefault();
			hexDraft = normalizedValue.toUpperCase();
			closePicker();
		}
	}

	function portal(node: HTMLElement) {
		document.body.appendChild(node);
		return {
			destroy() {
				node.remove();
			}
		};
	}

	function onWindowScroll() {
		if (open) updatePosition();
	}

	function onWindowResize() {
		if (open) updatePosition();
	}

	function onWindowMouseDown(e: MouseEvent) {
		if (!open) return;
		const target = e.target as Node | null;
		if (!target) return;
		if (triggerRef?.contains(target)) return;
		if (panelRef?.contains(target)) return;
		closePicker();
	}

	function onWindowMouseMove(e: MouseEvent) {
		if (!open) return;
		if (draggingSv) {
			updateFromSv(e.clientX, e.clientY);
			return;
		}
		if (draggingHue) {
			updateFromHue(e.clientX);
		}
	}

	function onWindowMouseUp() {
		draggingSv = false;
		draggingHue = false;
	}
</script>

<svelte:window
	onscroll={onWindowScroll}
	onresize={onWindowResize}
	onmousedown={onWindowMouseDown}
	onmousemove={onWindowMouseMove}
	onmouseup={onWindowMouseUp}
/>

<div class="relative w-full">
	<Button
		bind:ref={triggerRef}
		{id}
		type="button"
		variant="secondary"
		class="h-8 w-full justify-between"
		{disabled}
		onclick={togglePicker}
		aria-haspopup="dialog"
		aria-expanded={open}
	>
		<span class="flex min-w-0 items-center gap-2">
			<span
				class="input-highlight size-3.5 shrink-0 rounded-[3px]"
				style={`background:${normalizedValue};`}
			></span>
			<span class="truncate text-left text-[10px] text-foreground uppercase">
				{normalizedValue}
			</span>
		</span>
		<span class="ml-2 flex shrink-0 items-center text-foreground">
			<IconChevronsUpDown size={12} />
		</span>
	</Button>
</div>

{#if open}
	<div
		use:portal
		bind:this={panelRef}
		style={dropdownStyle}
		class="z-200 origin-top"
		in:scale={{ start: 0.98, duration: 120, easing: cubicInOut }}
		out:scale={{ start: 0.98, duration: 90, easing: cubicInOut }}
	>
		<div class="button-highlight w-full space-y-2 rounded-sm bg-dropdown p-2 shadow-md">
			<button
				bind:this={svRef}
				type="button"
				aria-label="Pick saturation and brightness"
				class="relative block h-24 w-full cursor-crosshair appearance-none overflow-hidden rounded-sm border border-frame-gray-200 bg-transparent p-0"
				onmousedown={startSvDrag}
			>
				<span
					class="pointer-events-none absolute inset-0"
					style={`background:hsl(${hsv.h}deg 100% 50%);`}
				></span>
				<span class="pointer-events-none absolute inset-0 bg-linear-to-r from-white to-transparent"
				></span>
				<span class="pointer-events-none absolute inset-0 bg-linear-to-t from-black to-transparent"
				></span>
				<span
					class="pointer-events-none absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white shadow-[0_0_0_1px_rgba(0,0,0,0.35)]"
					style={`left:${hsv.s * 100}%;top:${(1 - hsv.v) * 100}%;`}
				></span>
			</button>

			<button
				bind:this={hueRef}
				type="button"
				aria-label="Pick hue"
				class="input-highlight relative block h-2.5 w-full cursor-ew-resize appearance-none rounded-[3px] bg-transparent p-0"
				onmousedown={startHueDrag}
			>
				<span
					class="pointer-events-none absolute inset-0 rounded-[1.5px]"
					style="background:linear-gradient(90deg,#ff0000 0%,#ffff00 17%,#00ff00 33%,#00ffff 50%,#0000ff 67%,#ff00ff 83%,#ff0000 100%);"
				></span>
				<span
					class="button-highlight pointer-events-none absolute! top-1/2 z-200 h-4 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-[1.5px] bg-background"
					style={`left:${(hsv.h / 360) * 100}%;`}
				>
				</span>
			</button>

			<Input
				type="text"
				value={hexDraft}
				oninput={onHexInput}
				onblur={onHexBlur}
				onkeydown={onHexKeydown}
				spellcheck={false}
				autocomplete="off"
				placeholder="#FFFFFF"
				class="uppercase"
			/>
		</div>
	</div>
{/if}
