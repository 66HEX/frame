import { parse } from 'yaml';
import { sectionUiDefaults } from '$lib/site/content-ui';
import type { ContentFrontmatter } from './types';

const FRONTMATTER_BLOCK_RE = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/;

const metadataShape = {
	title: '',
	description: '',
	order: 0,
	sidebar: { label: '', hidden: false },
	showPagination: true,
	group: { label: '', order: 0 },
	view: {
		label: '',
		icon: '',
		order: 0,
		layout: '',
		ui: {
			...sectionUiDefaults,
			toc: {
				...sectionUiDefaults.toc,
				// Validate the element schema even when a fork has no default overrides.
				selectorOverrides: [{ slugPrefix: '', selector: '' }]
			}
		}
	},
	data: {}
};

function isRecord(value: unknown): value is Record<string, unknown> {
	return (
		value !== null &&
		typeof value === 'object' &&
		!Array.isArray(value) &&
		Object.getPrototypeOf(value) === Object.prototype
	);
}

function validateShape(value: unknown, shape: unknown, path: string, partial = true): void {
	if (Array.isArray(shape)) {
		if (!Array.isArray(value)) throw new Error(`${path} must be an array.`);
		value.forEach((item, index) => {
			validateShape(item, shape[0], `${path}[${String(index)}]`, false);
		});
	} else if (isRecord(shape)) {
		if (!isRecord(value)) throw new Error(`${path} must be an object.`);
		for (const key of Object.keys(value)) {
			if (!Object.hasOwn(shape, key))
				throw new Error(`Unknown metadata field ${path}.${key}. Put custom fields in data.`);
			if (key === 'data' && shape === metadataShape) {
				if (!isRecord(value[key])) throw new Error(`${path}.data must be an object.`);
				validateData(value[key], `${path}.data`);
			} else validateShape(value[key], shape[key], `${path}.${key}`, partial);
		}
		if (!partial)
			for (const key of Object.keys(shape)) {
				if (!Object.hasOwn(value, key)) throw new Error(`Missing ${path}.${key}.`);
			}
	} else if (
		typeof value !== typeof shape ||
		(typeof value === 'number' && !Number.isFinite(value))
	) {
		throw new Error(`${path} must be a ${typeof shape}.`);
	}
}

function validateData(value: unknown, path: string, ancestors = new Set<object>()): void {
	if (
		value === null ||
		typeof value === 'string' ||
		typeof value === 'boolean' ||
		(typeof value === 'number' && Number.isFinite(value))
	)
		return;
	if ((Array.isArray(value) || isRecord(value)) && !ancestors.has(value)) {
		ancestors.add(value);
		for (const [key, child] of Object.entries(value))
			validateData(child, `${path}.${key}`, ancestors);
		ancestors.delete(value);
		return;
	}
	throw new Error(`${path} must contain only serializable data, without circular references.`);
}

export function parseContentMetadata(value: unknown, source = 'content'): ContentFrontmatter {
	validateShape(value, metadataShape, source);
	const metadata = value as ContentFrontmatter;
	if (
		metadata.view?.layout !== undefined &&
		!['article', 'workspace'].includes(metadata.view.layout)
	) {
		throw new Error(`${source}.view.layout must be article or workspace.`);
	}
	return metadata;
}

/** Parse YAML using the same syntax supported by mdsvex, including multiline descriptions. */
export function parseContentSource(
	rawSource: string,
	source = 'content'
): {
	metadata: ContentFrontmatter;
	body: string;
} {
	const match = FRONTMATTER_BLOCK_RE.exec(rawSource);
	if (!match) return { metadata: {}, body: rawSource };
	let value: unknown;
	try {
		value = parse(match[1]);
	} catch (error) {
		throw new Error(`Invalid YAML in ${source}: ${String(error)}`, { cause: error });
	}
	return {
		metadata: parseContentMetadata(value ?? {}, source),
		body: rawSource.slice(match[0].length)
	};
}
