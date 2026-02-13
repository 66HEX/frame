import { browser } from '$app/environment';
import { init, register, locale } from 'svelte-i18n';
import { locale as osLocale } from '@tauri-apps/plugin-os';

const defaultLocale = 'en-US';

export const supportedLocales = [
	{ code: 'en-US', name: 'English', flag: '🇺🇸' },
	{ code: 'zh-CN', name: '简体中文', flag: '🇨🇳' },
	{ code: 'ja-JP', name: '日本語', flag: '🇯🇵' },
	{ code: 'ko-KR', name: '한국어', flag: '🇰🇷' },
	{ code: 'es-ES', name: 'Español', flag: '🇪🇸' },
	{ code: 'ru-RU', name: 'Русский', flag: '🇷🇺' },
	{ code: 'fr-FR', name: 'Français', flag: '🇫🇷' },
	{ code: 'de-DE', name: 'Deutsch', flag: '🇩🇪' },
	{ code: 'it-IT', name: 'Italiano', flag: '🇮🇹' }
] as const;

for (const loc of supportedLocales) {
	register(loc.code, () => import(`./locales/${loc.code}.json`));
}

function mapLocaleCode(localeStr: string | null): string {
	if (!localeStr) return defaultLocale;
	const matched = supportedLocales.find((l) => localeStr.startsWith(l.code.split('-')[0]));
	return matched?.code || defaultLocale;
}

function migrateOldLocale() {
	if (!browser) return;
	const stored = localStorage.getItem('locale');
	if (stored && (stored.includes('_') || !supportedLocales.some((l) => l.code === stored))) {
		localStorage.removeItem('locale');
	}
}

migrateOldLocale();

init({
	fallbackLocale: defaultLocale,
	initialLocale: defaultLocale
});

export async function initI18n() {
	if (!browser) return;

	const stored = localStorage.getItem('locale');
	if (stored && supportedLocales.some((l) => l.code === stored)) {
		locale.set(stored);
		return;
	}

	const systemLocale = await osLocale().catch(() => null);
	const mappedLocale = mapLocaleCode(systemLocale);
	locale.set(mappedLocale);
}

export function setLocale(newLocale: string) {
	locale.set(newLocale);
	if (browser) {
		localStorage.setItem('locale', newLocale);
	}
}

export { locale, t, _ } from 'svelte-i18n';
