import { readPreference, writePreference } from './preferences';
import { browser } from '$app/env';
import { contentUiDefaults } from '$lib/site/content-ui';

type Theme = 'light' | 'dark';

const storageKey = contentUiDefaults.theme.storageKey;

function isTheme(value: string | null): value is Theme {
	return value === 'light' || value === 'dark';
}

function applyTheme(theme: Theme) {
	if (!browser) {
		return;
	}

	const isDark = theme === 'dark';
	document.documentElement.classList.toggle('dark', isDark);
	document.documentElement.style.colorScheme = isDark ? 'dark' : 'light';
}

function getInitialTheme(): Theme {
	if (!browser) {
		return contentUiDefaults.theme.defaultMode === 'dark' ? 'dark' : 'light';
	}

	const storedTheme = readPreference(storageKey);
	if (isTheme(storedTheme)) {
		return storedTheme;
	}

	return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

function createThemeStore() {
	const initialTheme = getInitialTheme();
	let current = $state<Theme>(initialTheme);

	if (browser) {
		applyTheme(initialTheme);

		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const handlePreferredThemeChange = () => {
			const savedTheme = readPreference(storageKey);
			if (isTheme(savedTheme) || contentUiDefaults.theme.defaultMode !== 'system') {
				return;
			}

			const preferredTheme = mediaQuery.matches ? 'dark' : 'light';
			if (current !== preferredTheme) {
				current = preferredTheme;
				applyTheme(preferredTheme);
			}
		};

		mediaQuery.addEventListener('change', handlePreferredThemeChange);
	}

	const set = (theme: Theme) => {
		current = theme;
		if (browser) {
			writePreference(storageKey, theme);
		}
		applyTheme(theme);
	};

	const toggle = () => {
		set(current === 'dark' ? 'light' : 'dark');
	};

	return {
		get current() {
			return current;
		},
		get isDark() {
			return current === 'dark';
		},
		set,
		toggle
	};
}

export const themeStore = createThemeStore();
