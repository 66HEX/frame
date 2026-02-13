#!/usr/bin/env node
/**
 * Validates locale key parity, placeholder consistency, and source key coverage.
 * Usage: node scripts/i18n-check.mjs
 */
import path from 'node:path';
import {
	LOCALES_DIR,
	listLocaleFiles,
	readJson,
	flattenMessages,
	collectPlaceholders,
	loadGuardrailsConfig,
	extractKeysFromSource,
	keyIsIgnored
} from './i18n-common.mjs';

function formatList(values, max = 12) {
	if (values.length === 0) return '';
	const shown = values.slice(0, max);
	const lines = shown.map((value) => `  - ${value}`);
	if (values.length > max) {
		lines.push(`  - ... and ${values.length - max} more`);
	}
	return lines.join('\n');
}

function sameStringArray(left, right) {
	if (left.length !== right.length) return false;
	return left.every((value, index) => value === right[index]);
}

async function main() {
	const config = await loadGuardrailsConfig();
	const sourceLocaleFile = `${config.sourceLocale}.json`;
	const sourceLocalePath = path.join(LOCALES_DIR, sourceLocaleFile);
	const localeFiles = await listLocaleFiles();

	if (!localeFiles.includes(sourceLocaleFile)) {
		console.error(`Source locale file not found: ${sourceLocalePath}`);
		process.exit(1);
	}

	const sourceLocale = await readJson(sourceLocalePath);
	const sourceFlat = flattenMessages(sourceLocale);
	const sourceKeys = Object.keys(sourceFlat).sort();

	const extracted = await extractKeysFromSource(sourceKeys);
	const usedKeySet = new Set([
		...extracted.staticKeys,
		...extracted.templateResolvedKeys,
		...extracted.literalReferencedKeys
	]);

	const errors = [];
	const warnings = [];

	const usedButMissingInSource = extracted.staticKeys.filter((key) => !sourceKeys.includes(key)).sort();
	if (usedButMissingInSource.length > 0) {
		errors.push(
			[
				'Keys used in source code but missing in source locale:',
				formatList(usedButMissingInSource, 50)
			].join('\n')
		);
	}

	for (const localeFile of localeFiles) {
		if (localeFile === sourceLocaleFile) continue;
		const localeCode = localeFile.replace(/\.json$/, '');
		const localePath = path.join(LOCALES_DIR, localeFile);
		const localeObject = await readJson(localePath);
		const localeFlat = flattenMessages(localeObject);
		const localeKeys = Object.keys(localeFlat).sort();

		const missingKeys = sourceKeys.filter((key) => !(key in localeFlat));
		const extraKeys = localeKeys.filter((key) => !(key in sourceFlat));

		if (missingKeys.length > 0) {
			errors.push(
				[`[${localeCode}] Missing keys (${missingKeys.length}):`, formatList(missingKeys, 25)].join('\n')
			);
		}

		if (extraKeys.length > 0) {
			errors.push(
				[`[${localeCode}] Extra keys (${extraKeys.length}):`, formatList(extraKeys, 25)].join('\n')
			);
		}

		const placeholderMismatches = [];
		for (const key of sourceKeys) {
			const sourceValue = sourceFlat[key];
			const localeValue = localeFlat[key];

			if (typeof sourceValue !== 'string' || typeof localeValue !== 'string') continue;

			const sourcePlaceholders = collectPlaceholders(sourceValue);
			const localePlaceholders = collectPlaceholders(localeValue);

			if (!sameStringArray(sourcePlaceholders, localePlaceholders)) {
				placeholderMismatches.push(
					`${key} (source: ${JSON.stringify(sourcePlaceholders)}, locale: ${JSON.stringify(localePlaceholders)})`
				);
			}
		}

		if (placeholderMismatches.length > 0) {
			errors.push(
				[
					`[${localeCode}] Placeholder mismatches (${placeholderMismatches.length}):`,
					formatList(placeholderMismatches, 15)
				].join('\n')
			);
		}
	}

	const staleSourceKeys = sourceKeys.filter(
		(key) => !usedKeySet.has(key) && !keyIsIgnored(key, config)
	);

	if (staleSourceKeys.length > 0) {
		warnings.push(
			[
				`Potentially stale source locale keys (${staleSourceKeys.length}):`,
				formatList(staleSourceKeys, 20),
				'  (These are warnings only due to dynamic key access patterns.)'
			].join('\n')
		);
	}

	if (extracted.variableExpressions.length > 0) {
		warnings.push(
			[
				`Dynamic key expressions detected (${extracted.variableExpressions.length}):`,
				formatList(extracted.variableExpressions, 20),
				'  (Resolved via literal scans when possible.)'
			].join('\n')
		);
	}

	console.log(`Checked locales: ${localeFiles.length}`);
	console.log(`Source locale: ${config.sourceLocale}`);
	console.log(`Source files scanned: ${extracted.sourceFilesScanned}`);
	console.log(`Source locale keys: ${sourceKeys.length}`);
	console.log(`Used keys detected: ${usedKeySet.size}`);

	if (warnings.length > 0) {
		console.log('\nWarnings:');
		for (const warning of warnings) {
			console.log(`\n${warning}`);
		}
	}

	if (errors.length > 0) {
		console.error('\nErrors:');
		for (const error of errors) {
			console.error(`\n${error}`);
		}
		process.exit(1);
	}

	console.log('\nAll i18n guardrails passed.');
}

await main();
