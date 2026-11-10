<script lang="ts">
	import { themeStore } from '$lib/stores/theme.svelte';
	import { onMount } from 'svelte';
	import { ActionTooltip } from '$lib/components/action-tooltip';
	import { IconTransition } from '$lib/components/icon-transition';
	import { Button } from '$lib/components/ui/button';
	import { keyboardShortcuts, matchesKeyboardShortcut } from '$lib/site/keyboard-shortcuts';
	import { contentUiDefaults } from '$lib/site/content-ui';
	import { ThemeIcon } from '$lib/icons';

	const themeStorageKey = contentUiDefaults.theme.storageKey;

	const isDark = $derived(themeStore.isDark);
	let motionReady = $state(false);
	let tooltipLabel = $derived(
		isDark ? contentUiDefaults.theme.lightLabel : contentUiDefaults.theme.darkLabel
	);

	function applyTheme(dark: boolean) {
		themeStore.set(dark ? 'dark' : 'light');
	}

	function toggleTheme() {
		const nextIsDark = !isDark;
		const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

		if (document.startViewTransition && !prefersReducedMotion) {
			document.startViewTransition(() => applyTheme(nextIsDark));
		} else {
			applyTheme(nextIsDark);
		}

		try {
			localStorage.setItem(themeStorageKey, nextIsDark ? 'dark' : 'light');
		} catch {
			// The theme still changes when storage is unavailable.
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (!matchesKeyboardShortcut(event, keyboardShortcuts.theme)) return;

		event.preventDefault();
		toggleTheme();
	}

	onMount(() => {
		const enableMotionFrame = requestAnimationFrame(() => {
			motionReady = true;
		});

		function syncTheme(event: StorageEvent) {
			if (event.key === themeStorageKey) {
				applyTheme(event.newValue !== 'light');
			}
		}

		window.addEventListener('storage', syncTheme);
		return () => {
			cancelAnimationFrame(enableMotionFrame);
			window.removeEventListener('storage', syncTheme);
		};
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<ActionTooltip content={tooltipLabel} shortcut={keyboardShortcuts.theme.display} side="left">
	{#snippet trigger({ props })}
		<Button
			{...props}
			type="button"
			variant="ghost"
			size="icon-sm"
			aria-label={tooltipLabel}
			aria-keyshortcuts={keyboardShortcuts.theme.ariaKeyShortcuts}
			aria-pressed={isDark}
			onclick={toggleTheme}
		>
			<span class="theme-icon" class:motion-ready={motionReady}>
				<IconTransition
					active={isDark}
					inactiveIcon={ThemeIcon}
					activeIcon={ThemeIcon}
					class="size-full"
					iconClass="size-[18px]"
					size={18}
				/>
			</span>
		</Button>
	{/snippet}
</ActionTooltip>

<style>
	.theme-icon {
		display: flex;
		width: 1rem;
		height: 1rem;
		rotate: 0deg;
		transition-property: rotate;
		transition-duration: 180ms;
		transition-timing-function: cubic-bezier(0.2, 0, 0, 1);
	}

	.theme-icon:not(.motion-ready),
	.theme-icon:not(.motion-ready) :global(.icon-state) {
		transition: none;
	}

	:global(html.dark) .theme-icon {
		rotate: 180deg;
	}

	@media (prefers-reduced-motion: reduce) {
		.theme-icon {
			transition: none;
		}
	}
</style>
