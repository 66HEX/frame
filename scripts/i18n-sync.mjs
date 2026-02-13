import path from 'node:path';
import {
	LOCALES_DIR,
	listLocaleFiles,
	readJson,
	writeJson,
	loadGuardrailsConfig,
	syncLocaleTree
} from './i18n-common.mjs';

function hasFlag(flag) {
	return process.argv.includes(flag);
}

async function main() {
	const writeMode = hasFlag('--write');
	const keepExtra = hasFlag('--keep-extra');

	const config = await loadGuardrailsConfig();
	const sourceLocaleFile = `${config.sourceLocale}.json`;
	const sourceLocalePath = path.join(LOCALES_DIR, sourceLocaleFile);
	const localeFiles = await listLocaleFiles();

	if (!localeFiles.includes(sourceLocaleFile)) {
		console.error(`Source locale file not found: ${sourceLocalePath}`);
		process.exit(1);
	}

	const sourceLocale = await readJson(sourceLocalePath);
	let changedLocaleCount = 0;

	for (const localeFile of localeFiles) {
		if (localeFile === sourceLocaleFile) continue;
		const localeCode = localeFile.replace(/\.json$/, '');
		const localePath = path.join(LOCALES_DIR, localeFile);
		const currentLocale = await readJson(localePath);
		const stats = { added: [], removed: [] };
		const syncedLocale = syncLocaleTree(
			sourceLocale,
			currentLocale,
			localeCode,
			'',
			stats,
			keepExtra
		);

		const changed = stats.added.length > 0 || stats.removed.length > 0;
		if (!changed) {
			console.log(`[${localeCode}] already in sync`);
			continue;
		}

		changedLocaleCount += 1;

		console.log(`\n[${localeCode}] changes`);
		console.log(`- add missing: ${stats.added.length}`);
		console.log(`- remove extra: ${stats.removed.length}`);

		if (writeMode) {
			await writeJson(localePath, syncedLocale);
			console.log(`- wrote: ${localePath}`);
		}
	}

	if (!writeMode) {
		console.log('\nDry run only. Re-run with --write to apply changes.');
	}

	if (changedLocaleCount === 0) {
		console.log('\nNo locale changes required.');
	}
}

await main();
