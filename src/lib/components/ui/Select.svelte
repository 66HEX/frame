<script lang="ts">
	import { tick } from 'svelte';
	import { cubicInOut } from 'svelte/easing';
	import { scale } from 'svelte/transition';
	import Button from '$lib/components/ui/Button.svelte';
	import { cn } from '$lib/utils/cn';
	import { IconChevronsUpDown, IconCheck } from '$lib/icons';

	let {
		value = $bindable(''),
		options = [] as string[],
		placeholder = '',
		disabled = false,
		id,
		onchange
	}: {
		value?: string;
		options?: string[];
		placeholder?: string;
		disabled?: boolean;
		id?: string;
		onchange?: (value: string) => void;
	} = $props();

	let open = $state(false);
	let highlighted = $state(-1);

	let triggerRef: HTMLButtonElement | undefined = $state();
	let listRef: HTMLUListElement | undefined = $state();
	let dropdownStyle = $state('');

	const hasValue = $derived((value ?? '').trim().length > 0);
	const displayValue = $derived(hasValue ? (value ?? '') : placeholder);

	function updatePosition() {
		if (!triggerRef) return;
		const rect = triggerRef.getBoundingClientRect();
		dropdownStyle = `position:fixed;top:${rect.bottom + 4}px;left:${rect.left}px;width:${rect.width}px`;
	}

	async function openDropdown() {
		if (disabled || options.length === 0) return;
		updatePosition();
		open = true;
		const selectedIndex = options.indexOf(value ?? '');
		highlighted = selectedIndex >= 0 ? selectedIndex : 0;
		await tick();
		scrollHighlighted();
	}

	function closeDropdown() {
		open = false;
		highlighted = -1;
	}

	function commit(v: string) {
		value = v;
		onchange?.(v);
		closeDropdown();
	}

	function toggleDropdown() {
		if (disabled) return;
		if (open) {
			closeDropdown();
			return;
		}
		openDropdown();
	}

	function onTriggerKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			closeDropdown();
			return;
		}
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			if (!open) {
				openDropdown();
				return;
			}
			highlighted = Math.min(highlighted + 1, options.length - 1);
			scrollHighlighted();
			return;
		}
		if (e.key === 'ArrowUp') {
			e.preventDefault();
			if (!open) {
				openDropdown();
				return;
			}
			highlighted = Math.max(highlighted - 1, 0);
			scrollHighlighted();
			return;
		}
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			if (!open) {
				openDropdown();
				return;
			}
			if (highlighted >= 0 && highlighted < options.length) {
				commit(options[highlighted]);
			} else {
				closeDropdown();
			}
			return;
		}
		if (e.key === 'Tab') {
			closeDropdown();
		}
	}

	function scrollHighlighted() {
		if (highlighted < 0) return;
		(listRef?.children[highlighted] as HTMLElement | undefined)?.scrollIntoView({
			block: 'nearest'
		});
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
		if (listRef?.contains(target)) return;
		closeDropdown();
	}
</script>

<svelte:window
	onscroll={onWindowScroll}
	onresize={onWindowResize}
	onmousedown={onWindowMouseDown}
/>

<div class="relative w-full">
	<Button
		bind:ref={triggerRef}
		{id}
		type="button"
		variant="secondary"
		class="w-full justify-between"
		{disabled}
		onclick={toggleDropdown}
		onkeydown={onTriggerKeydown}
		aria-haspopup="listbox"
		aria-expanded={open}
	>
		<span class="truncate text-left text-foreground">{displayValue}</span>
		<span class="ml-2 flex shrink-0 items-center text-foreground">
			<IconChevronsUpDown size={12} />
		</span>
	</Button>
</div>

{#if open && options.length > 0}
	<div
		use:portal
		style={dropdownStyle}
		class="button-highlight z-200 origin-top rounded-sm"
		in:scale={{ start: 0.98, duration: 120, easing: cubicInOut }}
		out:scale={{ start: 0.98, duration: 90, easing: cubicInOut }}
	>
		<ul
			bind:this={listRef}
			class="max-h-48 overflow-y-auto rounded-sm bg-dropdown shadow-md [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
			role="listbox"
		>
			{#each options as option, i (option)}
				{@const isSelected = option === value}
				{@const isHighlighted = i === highlighted}
				<li
					role="option"
					aria-selected={isSelected}
					tabindex="-1"
					onmousedown={(e) => {
						e.preventDefault();
						commit(option);
					}}
					onmouseenter={() => (highlighted = i)}
					class={cn(
						'flex h-7 w-full items-center justify-between px-3 text-[10px] text-frame-gray-600',
						isHighlighted && 'bg-frame-gray-100 text-foreground',
						isSelected && 'text-foreground'
					)}
				>
					<span class="truncate">{option}</span>
					{#if isSelected}
						<IconCheck size={12} class="mb-1 ml-2 shrink-0" />
					{/if}
				</li>
			{/each}
		</ul>
	</div>
{/if}
