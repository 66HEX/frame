import path from 'node:path';
import {
	LOCALES_DIR,
	listLocaleFiles,
	readJson,
	flattenMessages,
	loadGuardrailsConfig,
	extractKeysFromSource
} from './i18n-common.mjs';

function printArray(title, values) {
	console.log(`\n${title} (${values.length})`);
	for (const value of values) {
		console.log(`- ${value}`);
	}
}

async function main() {
	const config = await loadGuardrailsConfig();
	const localeFileName = `${config.sourceLocale}.json`;
	const localePath = path.join(LOCALES_DIR, localeFileName);

	const localeFiles = await listLocaleFiles();
	if (!localeFiles.includes(localeFileName)) {
		console.error(`Source locale file not found: ${localePath}`);
		process.exit(1);
	}

	const sourceLocale = await readJson(localePath);
	const baseKeySet = Object.keys(flattenMessages(sourceLocale)).sort();

	const extracted = await extractKeysFromSource(baseKeySet);

	const usedKeySet = new Set([
		...extracted.staticKeys,
		...extracted.templateResolvedKeys,
		...extracted.literalReferencedKeys
	]);

	const missingInSourceLocale = extracted.staticKeys.filter((key) => !baseKeySet.includes(key)).sort();

	const output = {
		sourceLocale: config.sourceLocale,
		sourceFilesScanned: extracted.sourceFilesScanned,
		keyCounts: {
			sourceLocaleKeys: baseKeySet.length,
			usedKeys: usedKeySet.size,
			staticCallKeys: extracted.staticKeys.length,
			templateResolvedKeys: extracted.templateResolvedKeys.length,
			literalReferencedKeys: extracted.literalReferencedKeys.length
		},
		staticCallKeys: extracted.staticKeys,
		templatePatterns: extracted.templatePatterns,
		templateResolvedKeys: extracted.templateResolvedKeys,
		literalReferencedKeys: extracted.literalReferencedKeys,
		variableExpressions: extracted.variableExpressions,
		missingInSourceLocale
	};

	console.log(JSON.stringify(output, null, 2));

	if (missingInSourceLocale.length > 0) {
		printArray('Keys used in source code but missing in source locale', missingInSourceLocale);
		process.exitCode = 1;
	}
}

await main();
