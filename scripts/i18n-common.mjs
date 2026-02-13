import fs from 'node:fs/promises';
import path from 'node:path';

export const LOCALES_DIR = path.resolve(process.cwd(), 'src/lib/i18n/locales');
export const SOURCE_DIR = path.resolve(process.cwd(), 'src');
export const GUARDRAILS_CONFIG_PATH = path.resolve(process.cwd(), 'src/lib/i18n/guardrails.json');

export const DEFAULT_CONFIG = {
	sourceLocale: 'en-US',
	ignoredUnusedKeyPrefixes: ['_meta.'],
	ignoredUnusedKeys: []
};

function isPlainObject(value) {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function flattenMessages(input, prefix = '', output = {}) {
	if (!isPlainObject(input)) {
		if (prefix) {
			output[prefix] = input;
		}
		return output;
	}

	for (const [key, value] of Object.entries(input)) {
		const nextPrefix = prefix ? `${prefix}.${key}` : key;
		if (isPlainObject(value)) {
			flattenMessages(value, nextPrefix, output);
		} else {
			output[nextPrefix] = value;
		}
	}

	return output;
}

export async function readJson(filePath) {
	const raw = await fs.readFile(filePath, 'utf8');
	return JSON.parse(raw);
}

export async function writeJson(filePath, value) {
	const serialized = `${JSON.stringify(value, null, '\t')}\n`;
	await fs.writeFile(filePath, serialized, 'utf8');
}

export async function listLocaleFiles() {
	const entries = await fs.readdir(LOCALES_DIR, { withFileTypes: true });
	return entries
		.filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
		.map((entry) => entry.name)
		.sort();
}

export async function loadGuardrailsConfig() {
	try {
		const config = await readJson(GUARDRAILS_CONFIG_PATH);
		return {
			sourceLocale: config.sourceLocale || DEFAULT_CONFIG.sourceLocale,
			ignoredUnusedKeyPrefixes: Array.isArray(config.ignoredUnusedKeyPrefixes)
				? config.ignoredUnusedKeyPrefixes
				: DEFAULT_CONFIG.ignoredUnusedKeyPrefixes,
			ignoredUnusedKeys: Array.isArray(config.ignoredUnusedKeys)
				? config.ignoredUnusedKeys
				: DEFAULT_CONFIG.ignoredUnusedKeys
		};
	} catch {
		return DEFAULT_CONFIG;
	}
}

export function collectPlaceholders(value) {
	if (typeof value !== 'string') return [];
	const placeholders = new Set();
	const regex = /{([a-zA-Z0-9_]+)}/g;
	let match = regex.exec(value);
	while (match) {
		placeholders.add(match[1]);
		match = regex.exec(value);
	}
	return Array.from(placeholders).sort();
}

function wildcardToRegex(pattern) {
	const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	return new RegExp(`^${escaped.replace(/\\\*/g, '[^.]+')}$`);
}

function extractStaticCallKeys(content, collector) {
	const literalCallPatterns = [
		/\$_\(\s*(['"])([^'"`]+)\1/g,
		/(?:^|[^\w$.])t\(\s*(['"])([^'"`]+)\1/gm
	];

	for (const pattern of literalCallPatterns) {
		let match = pattern.exec(content);
		while (match) {
			collector.staticKeys.add(match[2]);
			match = pattern.exec(content);
		}
	}
}

function extractTemplateCallKeys(content, collector, baseKeys) {
	const templateCallPatterns = [/\$_\(\s*`([^`]+)`/g, /(?:^|[^\w$.])t\(\s*`([^`]+)`/gm];

	for (const pattern of templateCallPatterns) {
		let match = pattern.exec(content);
		while (match) {
			const templateBody = match[1].trim();
			if (templateBody.includes('${')) {
				const wildcard = templateBody.replace(/\$\{[^}]+\}/g, '*');
				collector.templatePatterns.add(wildcard);
				const regex = wildcardToRegex(wildcard);
				for (const key of baseKeys) {
					if (regex.test(key)) {
						collector.templateResolvedKeys.add(key);
					}
				}
			} else {
				collector.staticKeys.add(templateBody);
			}
			match = pattern.exec(content);
		}
	}
}

function extractVariableCallExpressions(content, collector) {
	const variableCallPatterns = [
		/\$_\(\s*([A-Za-z_$][\w$.]*)\s*(?:,|\))/g,
		/(?:^|[^\w$.])t\(\s*([A-Za-z_$][\w$.]*)\s*(?:,|\))/gm
	];

	for (const pattern of variableCallPatterns) {
		let match = pattern.exec(content);
		while (match) {
			collector.variableExpressions.add(match[1]);
			match = pattern.exec(content);
		}
	}
}

function extractLiteralKeyReferences(content, collector, baseKeySet) {
	const keyLikeString = /(['"])([A-Za-z0-9]+(?:\.[A-Za-z0-9_-]+)+)\1/g;
	let match = keyLikeString.exec(content);
	while (match) {
		const literalValue = match[2];
		if (baseKeySet.has(literalValue)) {
			collector.literalReferencedKeys.add(literalValue);
		}
		match = keyLikeString.exec(content);
	}
}

async function walkFilesRecursively(directory, extensions, output = []) {
	const entries = await fs.readdir(directory, { withFileTypes: true });
	for (const entry of entries) {
		if (entry.name.startsWith('.')) continue;
		const absolutePath = path.join(directory, entry.name);
		if (entry.isDirectory()) {
			await walkFilesRecursively(absolutePath, extensions, output);
			continue;
		}
		if (extensions.has(path.extname(entry.name))) {
			output.push(absolutePath);
		}
	}
	return output;
}

export async function extractKeysFromSource(baseKeys) {
	const baseKeySet = new Set(baseKeys);
	const sourceFiles = await walkFilesRecursively(SOURCE_DIR, new Set(['.svelte', '.ts', '.js']));
	const collector = {
		staticKeys: new Set(),
		templatePatterns: new Set(),
		templateResolvedKeys: new Set(),
		variableExpressions: new Set(),
		literalReferencedKeys: new Set()
	};

	for (const filePath of sourceFiles) {
		const content = await fs.readFile(filePath, 'utf8');
		extractStaticCallKeys(content, collector);
		extractTemplateCallKeys(content, collector, baseKeys);
		extractVariableCallExpressions(content, collector);
		extractLiteralKeyReferences(content, collector, baseKeySet);
	}

	return {
		sourceFilesScanned: sourceFiles.length,
		staticKeys: Array.from(collector.staticKeys).sort(),
		templatePatterns: Array.from(collector.templatePatterns).sort(),
		templateResolvedKeys: Array.from(collector.templateResolvedKeys).sort(),
		variableExpressions: Array.from(collector.variableExpressions).sort(),
		literalReferencedKeys: Array.from(collector.literalReferencedKeys).sort()
	};
}

export function keyIsIgnored(key, config) {
	if (config.ignoredUnusedKeys.includes(key)) return true;
	return config.ignoredUnusedKeyPrefixes.some((prefix) => key.startsWith(prefix));
}

export function syncLocaleTree(baseNode, localeNode, localeCode, pathPrefix, stats, keepExtra) {
	if (isPlainObject(baseNode)) {
		const nextNode = {};
		const localeObject = isPlainObject(localeNode) ? localeNode : {};

		for (const [key, childBase] of Object.entries(baseNode)) {
			const nextPath = pathPrefix ? `${pathPrefix}.${key}` : key;
			if (Object.prototype.hasOwnProperty.call(localeObject, key)) {
				nextNode[key] = syncLocaleTree(
					childBase,
					localeObject[key],
					localeCode,
					nextPath,
					stats,
					keepExtra
				);
			} else {
				stats.added.push(nextPath);
				nextNode[key] = syncLocaleTree(childBase, undefined, localeCode, nextPath, stats, keepExtra);
			}
		}

		for (const key of Object.keys(localeObject)) {
			if (Object.prototype.hasOwnProperty.call(baseNode, key)) continue;
			const nextPath = pathPrefix ? `${pathPrefix}.${key}` : key;
			if (keepExtra) {
				nextNode[key] = localeObject[key];
			} else {
				stats.removed.push(nextPath);
			}
		}

		return nextNode;
	}

	if (localeNode === undefined) {
		if (typeof baseNode === 'string' && !pathPrefix.startsWith('_meta.')) {
			return `[TODO ${localeCode}] ${baseNode}`;
		}
		return baseNode;
	}

	return localeNode;
}
