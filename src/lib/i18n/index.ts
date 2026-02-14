import { browser } from '$app/environment';
import { init, register, locale } from 'svelte-i18n';
import { locale as osLocale } from '@tauri-apps/plugin-os';

const defaultLocale = 'en-US';

const localeModules = import.meta.glob('./locales/*.json', { eager: true }) as Record<
	string,
	{ default: Record<string, unknown> & { _meta: { name: string; flag: string } } }
>;

export const supportedLocales = Object.entries(localeModules).map(([path, module]) => {
	const code = path.replace('./locales/', '').replace('.json', '');
	const meta = module.default._meta;
	return { code, name: meta.name, flag: meta.flag };
});

for (const loc of supportedLocales) {
	const path = `./locales/${loc.code}.json`;
	const module = localeModules[path];

	if (module) {
		register(loc.code, async () => module.default);
	} else {
		console.error(`Missing locale module: ${path}`);
	}
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
