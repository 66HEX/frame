#!/usr/bin/env node
/**
 * Translates locale entries from the source locale via DeepL.
 * Usage:
 *   DEEPL_API_KEY=... node scripts/i18n-translate.mjs [--write] [--rewrite-existing]
 *   DEEPL_API_KEY=... node scripts/i18n-translate.mjs [--write] [--locale=de-DE,fr-FR]
 */
import path from 'node:path';
import {
	LOCALES_DIR,
	listLocaleFiles,
	readJson,
	writeJson,
	flattenMessages,
	loadGuardrailsConfig
} from './i18n-common.mjs';

const TODO_PREFIX = /^\[TODO [^\]]+\]\s*/;
const DEFAULT_BATCH_SIZE = 40;

function hasFlag(flag) {
	return process.argv.includes(flag);
}

function readArg(name) {
	const prefix = `${name}=`;
	const match = process.argv.find((arg) => arg.startsWith(prefix));
	return match ? match.slice(prefix.length) : null;
}

function printHelp() {
	console.log(`DeepL i18n translation helper

Required env:
  DEEPL_API_KEY   DeepL API key

Optional env:
  DEEPL_API_URL   Override API URL (defaults to api-free/api based on key type)

Flags:
  --write             Write translated locales to disk (default: dry-run)
  --rewrite-existing  Re-translate all translatable keys (not only TODO/missing)
  --locale=...        Comma-separated locale list (e.g. --locale=de-DE,fr-FR)
  --batch-size=...    Texts per API request (default: ${DEFAULT_BATCH_SIZE})
  --help              Show this help
`);
}

function inferDeepLApiUrl(authKey) {
	if (process.env.DEEPL_API_URL) return process.env.DEEPL_API_URL;
	return authKey.endsWith(':fx')
		? 'https://api-free.deepl.com/v2/translate'
		: 'https://api.deepl.com/v2/translate';
}

function mapLocaleToDeepLTarget(localeCode) {
	const primary = localeCode.split('-')[0]?.toUpperCase();
	switch (primary) {
		case 'DE':
			return 'DE';
		case 'ES':
			return 'ES';
		case 'FR':
			return 'FR';
		case 'IT':
			return 'IT';
		case 'JA':
			return 'JA';
		case 'KO':
			return 'KO';
		case 'RU':
			return 'RU';
		case 'ZH':
			return 'ZH';
		default:
			return null;
	}
}

function mapSourceLocaleToDeepL(localeCode) {
	const primary = localeCode.split('-')[0]?.toUpperCase();
	if (primary === 'EN') return 'EN';
	return primary || 'EN';
}

function splitIntoChunks(values, size) {
	const output = [];
	for (let index = 0; index < values.length; index += size) {
		output.push(values.slice(index, index + size));
	}
	return output;
}

function getValueAtPath(object, dottedPath) {
	const parts = dottedPath.split('.');
	let cursor = object;
	for (const part of parts) {
		if (!cursor || typeof cursor !== 'object' || !(part in cursor)) return undefined;
		cursor = cursor[part];
	}
	return cursor;
}

function setValueAtPath(object, dottedPath, value) {
	const parts = dottedPath.split('.');
	let cursor = object;
	for (let index = 0; index < parts.length - 1; index += 1) {
		const part = parts[index];
		if (typeof cursor[part] !== 'object' || cursor[part] === null || Array.isArray(cursor[part])) {
			cursor[part] = {};
		}
		cursor = cursor[part];
	}
	cursor[parts[parts.length - 1]] = value;
}

function escapeXml(value) {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&apos;');
}

function unescapeXml(value) {
	return value
		.replaceAll('&lt;', '<')
		.replaceAll('&gt;', '>')
		.replaceAll('&quot;', '"')
		.replaceAll('&apos;', "'")
		.replaceAll('&amp;', '&');
}

function encodePlaceholders(value) {
	const escaped = escapeXml(value);
	return escaped.replace(/{([a-zA-Z0-9_]+)}/g, '<ph id="$1"/>');
}

function encodePlaceholdersFallback(value) {
	return value.replace(/{([a-zA-Z0-9_]+)}/g, '__DEEPL_PH_$1__');
}

function decodePlaceholders(value) {
	const withPlaceholders = value
		.replace(/<ph\s+id="([^"]+)"\s*\/>/g, '{$1}')
		.replace(/&lt;ph\s+id=&quot;([^&]+)&quot;\s*\/&gt;/g, '{$1}')
		.replace(/__DEEPL_PH_([A-Za-z0-9_]+)__/g, '{$1}');
	return unescapeXml(withPlaceholders);
}

async function requestDeepLTranslation({
	apiUrl,
	authKey,
	sourceLang,
	targetLang,
	texts,
	maxRetries = 3
}) {
	let useTagHandling = true;

	for (let attempt = 1; attempt <= maxRetries; attempt += 1) {
		const params = new URLSearchParams();
		params.append('source_lang', sourceLang);
		params.append('target_lang', targetLang);
		params.append('preserve_formatting', '1');
		params.append('split_sentences', 'nonewlines');
		if (useTagHandling) {
			params.append('tag_handling', 'xml');
			params.append('ignore_tags', 'ph');
		}

		for (const text of texts) {
			params.append(
				'text',
				useTagHandling ? encodePlaceholders(text) : encodePlaceholdersFallback(text)
			);
		}

		const response = await fetch(apiUrl, {
			method: 'POST',
			headers: {
				Authorization: `DeepL-Auth-Key ${authKey}`,
				'Content-Type': 'application/x-www-form-urlencoded'
			},
			body: params.toString()
		});

		if (!response.ok) {
			const body = await response.text();
			const parseError =
				response.status === 400 && body.toLowerCase().includes('tag handling parsing failed');
			if (useTagHandling && parseError) {
				useTagHandling = false;
				attempt -= 1;
				continue;
			}

			const canRetry = response.status === 429 || response.status >= 500;
			if (attempt < maxRetries && canRetry) {
				const delayMs = 400 * Math.pow(2, attempt - 1);
				await new Promise((resolve) => setTimeout(resolve, delayMs));
				continue;
			}
			throw new Error(`DeepL request failed (${response.status}): ${body.slice(0, 500)}`);
		}

		const json = await response.json();
		const translations = Array.isArray(json?.translations) ? json.translations : null;
		if (!translations || translations.length !== texts.length) {
			throw new Error('DeepL response shape mismatch.');
		}
		return translations.map((entry) => decodePlaceholders(String(entry.text ?? '')));
	}

	throw new Error('DeepL request failed after retries.');
}

async function main() {
	if (hasFlag('--help')) {
		printHelp();
		return;
	}

	const writeMode = hasFlag('--write');
	const rewriteExisting = hasFlag('--rewrite-existing');
	const localesArg = readArg('--locale');
	const requestedLocales = localesArg
		? new Set(
				localesArg
					.split(',')
					.map((value) => value.trim())
					.filter(Boolean)
			)
		: null;

	const batchSizeRaw = readArg('--batch-size');
	const batchSize = batchSizeRaw ? Number.parseInt(batchSizeRaw, 10) : DEFAULT_BATCH_SIZE;
	if (!Number.isInteger(batchSize) || batchSize <= 0) {
		console.error(`Invalid --batch-size value: ${batchSizeRaw}`);
		process.exit(1);
	}

	const authKey = process.env.DEEPL_API_KEY;
	if (!authKey) {
		console.error('Missing DEEPL_API_KEY. Export it and retry.');
		process.exit(1);
	}

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
	const sourceLang = mapSourceLocaleToDeepL(config.sourceLocale);
	const apiUrl = inferDeepLApiUrl(authKey);

	const targetLocaleFiles = localeFiles.filter((file) => {
		if (file === sourceLocaleFile) return false;
		const code = file.replace(/\.json$/, '');
		if (!requestedLocales) return true;
		return requestedLocales.has(code);
	});

	if (targetLocaleFiles.length === 0) {
		console.log('No target locales selected.');
		return;
	}

	console.log(`Source locale: ${config.sourceLocale}`);
	console.log(`Target locales: ${targetLocaleFiles.length}`);
	console.log(
		`Mode: ${writeMode ? 'write' : 'dry-run'}${rewriteExisting ? ' + rewrite-existing' : ''}`
	);

	let translatedLocaleCount = 0;

	for (const localeFile of targetLocaleFiles) {
		const localeCode = localeFile.replace(/\.json$/, '');
		const targetLang = mapLocaleToDeepLTarget(localeCode);
		if (!targetLang) {
			console.log(`[${localeCode}] skipped (unsupported DeepL target language)`);
			continue;
		}

		const localePath = path.join(LOCALES_DIR, localeFile);
		const currentLocale = await readJson(localePath);
		const updatedLocale = JSON.parse(JSON.stringify(currentLocale));
		const currentFlat = flattenMessages(currentLocale);

		const keysToTranslate = [];
		for (const key of sourceKeys) {
			if (key.startsWith('_meta.')) continue;
			const sourceValue = sourceFlat[key];
			if (typeof sourceValue !== 'string') continue;

			const existingValue = currentFlat[key];
			const needsTranslate = rewriteExisting
				? true
				: existingValue === undefined ||
					(typeof existingValue === 'string' && TODO_PREFIX.test(existingValue));

			if (needsTranslate) {
				keysToTranslate.push(key);
			}
		}

		if (keysToTranslate.length === 0) {
			console.log(`[${localeCode}] nothing to translate`);
			continue;
		}

		console.log(`[${localeCode}] translating ${keysToTranslate.length} keys...`);

		const keyChunks = splitIntoChunks(keysToTranslate, batchSize);
		for (const keyChunk of keyChunks) {
			const sourceTexts = keyChunk.map((key) => String(sourceFlat[key]));
			const translatedTexts = await requestDeepLTranslation({
				apiUrl,
				authKey,
				sourceLang,
				targetLang,
				texts: sourceTexts
			});

			for (let index = 0; index < keyChunk.length; index += 1) {
				const key = keyChunk[index];
				const translated = translatedTexts[index];
				if (!translated || !translated.trim()) continue;
				setValueAtPath(updatedLocale, key, translated);
			}
		}

		const changedKeys = keysToTranslate.filter((key) => {
			const before = getValueAtPath(currentLocale, key);
			const after = getValueAtPath(updatedLocale, key);
			return before !== after;
		});

		if (changedKeys.length === 0) {
			console.log(`[${localeCode}] no changes after translation`);
			continue;
		}

		if (writeMode) {
			await writeJson(localePath, updatedLocale);
			console.log(`[${localeCode}] wrote ${changedKeys.length} keys`);
		} else {
			console.log(`[${localeCode}] would update ${changedKeys.length} keys`);
		}

		translatedLocaleCount += 1;
	}

	if (!writeMode) {
		console.log('\nDry run only. Re-run with --write to apply changes.');
	}

	if (translatedLocaleCount === 0) {
		console.log('\nNo locale files changed.');
	}
}

await main();
