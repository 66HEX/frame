<script lang="ts">
	import { untrack, tick } from 'svelte';
	import { cn } from '$lib/utils/cn';

	let {
		value = 0,
		onchange,
		disabled = false,
		class: className
	}: {
		value: number;
		onchange: (newValue: number) => void;
		disabled?: boolean;
		class?: string;
	} = $props();

	function secondsToTimecode(totalSeconds: number): string {
		const h = Math.floor(totalSeconds / 3600);
		const m = Math.floor((totalSeconds % 3600) / 60);
		const s = Math.floor(totalSeconds % 60);
		const ms = Math.round((totalSeconds % 1) * 1000);

		return (
			h.toString().padStart(2, '0') +
			':' +
			m.toString().padStart(2, '0') +
			':' +
			s.toString().padStart(2, '0') +
			'.' +
			ms.toString().padStart(3, '0')
		);
	}

	function timecodeToSeconds(timecode: string): number {
		const [hms, msStr] = timecode.split('.');
		const parts = hms.split(':').map(Number);
		const ms = Number(msStr || '0');
		if (parts.length !== 3) return 0;
		return parts[0] * 3600 + parts[1] * 60 + parts[2] + ms / 1000;
	}

	function parsePastedTimecode(raw: string): number | null {
		const value = raw.trim().replace(',', '.');
		if (!value) return null;

		const directSeconds = Number(value);
		if (Number.isFinite(directSeconds) && directSeconds >= 0) {
			return directSeconds;
		}

		const [hms, msPart = '0'] = value.split('.');
		const parts = hms.split(':');
		if (parts.length < 1 || parts.length > 3) return null;
		if (!parts.every((p) => /^\d+$/.test(p))) return null;
		if (!/^\d+$/.test(msPart)) return null;

		let hours = 0;
		let minutes = 0;
		let seconds = 0;

		if (parts.length === 3) {
			hours = Number(parts[0]);
			minutes = Number(parts[1]);
			seconds = Number(parts[2]);
		} else if (parts.length === 2) {
			minutes = Number(parts[0]);
			seconds = Number(parts[1]);
		} else {
			seconds = Number(parts[0]);
		}

		if (!Number.isFinite(hours) || !Number.isFinite(minutes) || !Number.isFinite(seconds)) {
			return null;
		}
		if (minutes > 59 || seconds > 59) return null;

		const milliseconds = Number(msPart);
		const fractional = milliseconds / Math.pow(10, msPart.length);
		return hours * 3600 + minutes * 60 + seconds + fractional;
	}

	let displayValue = $state(untrack(() => secondsToTimecode(value)));
	let inputRef: HTMLInputElement | undefined = $state();

	$effect(() => {
		if (Math.abs(timecodeToSeconds(untrack(() => displayValue)) - value) > 0.001) {
			displayValue = secondsToTimecode(value);
		}
	});

	async function handleKeyDown(e: KeyboardEvent) {
		if (disabled) {
			e.preventDefault();
			return;
		}
		const input = e.target as HTMLInputElement;
		let cursor = input.selectionStart || 0;
		const key = e.key;

		if (['ArrowLeft', 'ArrowRight'].includes(key)) {
			if (key === 'ArrowLeft') {
				e.preventDefault();
				let newCursor = cursor - 1;
				if (newCursor < 0) newCursor = 0;
				if (displayValue[newCursor] === ':' || displayValue[newCursor] === '.') {
					newCursor--;
				}
				input.setSelectionRange(newCursor, newCursor);
			} else if (key === 'ArrowRight') {
				e.preventDefault();
				let newCursor = cursor + 1;
				if (newCursor > displayValue.length) newCursor = displayValue.length;
				if (displayValue[newCursor - 1] === ':' || displayValue[newCursor - 1] === '.') {
					newCursor++;
				}
				input.setSelectionRange(newCursor, newCursor);
			}
			return;
		}

		if (['Backspace', 'Delete'].includes(key) || /^[0-9]$/.test(key)) {
			e.preventDefault();
		} else {
			if (!e.metaKey && !e.ctrlKey && !e.altKey && key.length === 1) {
				e.preventDefault();
			}
			return;
		}

		let chars = displayValue.split('');

		if (/^[0-9]$/.test(key)) {
			if (cursor >= chars.length) return;

			if (chars[cursor] === ':' || chars[cursor] === '.') {
				cursor++;
			}
			if (cursor >= chars.length) return;

			chars[cursor] = key;
			displayValue = chars.join('');

			let newCursor = cursor + 1;
			if (newCursor < chars.length && (chars[newCursor] === ':' || chars[newCursor] === '.')) {
				newCursor++;
			}

			await tick();
			input.setSelectionRange(newCursor, newCursor);
			onchange(timecodeToSeconds(displayValue));
		} else if (key === 'Backspace') {
			let target = cursor - 1;
			if (target < 0) return;

			if (chars[target] === ':' || chars[target] === '.') {
				target--;
			}
			if (target < 0) return;

			chars[target] = '0';
			displayValue = chars.join('');

			await tick();
			input.setSelectionRange(target, target);
			onchange(timecodeToSeconds(displayValue));
		} else if (key === 'Delete') {
			let target = cursor;
			if (target >= chars.length) return;

			if (chars[target] === ':' || chars[target] === '.') {
				target++;
			}
			if (target >= chars.length) return;

			chars[target] = '0';
			displayValue = chars.join('');

			await tick();
			input.setSelectionRange(target, target);
			onchange(timecodeToSeconds(displayValue));
		}
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault();
		if (disabled) return;
		const pasted = e.clipboardData?.getData('text') ?? '';
		const seconds = parsePastedTimecode(pasted);
		if (seconds === null) return;
		const normalized = Math.max(0, seconds);
		displayValue = secondsToTimecode(normalized);
		onchange(normalized);
	}
</script>

<input
	bind:this={inputRef}
	type="text"
	value={displayValue}
	{disabled}
	class={cn(
		'flex h-7.5 w-full rounded-sm border border-gray-alpha-200 bg-transparent px-3 py-1.5 text-[11px] tracking-wide transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-gray-alpha-400 focus-visible:border-blue-600! focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
		className
	)}
	onkeydown={handleKeyDown}
	onpaste={handlePaste}
	oncontextmenu={(e) => e.preventDefault()}
	spellcheck="false"
	autocomplete="off"
/>
