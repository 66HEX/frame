import * as icons from '$lib/icons';
import type { ContentItem, ContentSectionConfig } from './types';
import { buildContentRegistry, type RegisteredPage } from './registry';
import { mergeSectionUiConfig, type SectionUiConfig } from '$lib/site/content-ui';
import { parseContentSource } from './frontmatter';
import { getAdjacentItems, getHref } from './manifest';
import GithubSlugger from 'github-slugger';
import type { Component } from 'svelte';

export type ContentSectionId = string;
export type ContentMetadata = {
	href: string;
	slug: string;
	title: string;
	description?: string;
	sourceType: 'markdown' | 'svelte';
};
export type ContentTocHeading = { id: string; text: string; level: number };
export type ContentModule = { default: Component; metadata?: Record<string, unknown> };

const prefix = '/src/lib/site/content/';
const allMarkdownRaw = import.meta.glob<string>(
	[
		'/src/lib/site/content/**/*.svx',
		'/src/lib/site/content/**/*.mdx',
		'/src/lib/site/content/**/*.md'
	],
	{ query: '?raw', eager: true, import: 'default' }
);
const allMarkdownModules = import.meta.glob<ContentModule>([
	'/src/lib/site/content/**/*.svx',
	'/src/lib/site/content/**/*.mdx',
	'/src/lib/site/content/**/*.md',
	'!/src/lib/site/content/**/_meta.*'
]);
const allSveltePages = import.meta.glob<ContentModule>('/src/lib/site/content/**/*.svelte', {
	eager: true
});
const registry = buildContentRegistry([
	...Object.entries(allMarkdownRaw).map(([path, raw]) => {
		const { metadata, body } = parseContentSource(raw, path);
		if (/\/_meta\.(svx|mdx|md)$/.test(path) && body.trim())
			throw new Error(`${path} must contain frontmatter only. Use index for a landing page.`);
		return { path: path.slice(prefix.length), metadata };
	}),
	...Object.entries(allSveltePages).map(([path, { metadata }]) => ({
		path: path.slice(prefix.length),
		metadata
	}))
]);

export const contentSections: ContentSectionConfig[] = registry.map((view) => {
	if (!Object.hasOwn(icons, view.icon))
		throw new Error(`Unknown icon ${view.icon} in ${view.id}/index. Use an export from lib/icons.`);
	return {
		id: view.id,
		label: view.label,
		layout: view.layout,
		ui: view.ui,
		icon: icons[view.icon as keyof typeof icons]
	};
});
const sectionsById = new Map(contentSections.map((section) => [section.id, section]));
const pagesBySection = new Map(registry.map((view) => [view.id, view.pages]));
const pagesByRoute = new Map(
	registry.flatMap((view) =>
		view.pages.map((page) => [getHref(`/${view.id}`, page.slug), page] as const)
	)
);
const manifests = new Map(
	registry.map((view) => [
		view.id,
		view.pages.map(
			({ slug, name, category, categoryId, sidebarHidden, showPagination }): ContentItem => ({
				slug,
				name,
				category,
				categoryId,
				sidebarHidden,
				showPagination
			})
		)
	])
);

export function getContentSectionConfig(sectionId: ContentSectionId) {
	return sectionsById.get(sectionId);
}
export function getContentSectionUiConfig(sectionId: ContentSectionId): SectionUiConfig {
	return mergeSectionUiConfig(sectionsById.get(sectionId)?.ui);
}
/** All routable pages, including pages hidden from the sidebar. */
export function getContentSectionManifest(sectionId: ContentSectionId): ContentItem[] {
	return manifests.get(sectionId) ?? [];
}
export function getContentSectionPages(sectionId: ContentSectionId): readonly RegisteredPage[] {
	return pagesBySection.get(sectionId) ?? [];
}
export function getContentSectionSlug(sectionId: ContentSectionId, pathname: string) {
	const normalized = pathname.replace(/\/+$/, '');
	return normalized === `/${sectionId}`
		? ''
		: normalized.replace(new RegExp(`^/${sectionId}/`), '');
}
export function getContentSectionMetadata(
	sectionId: ContentSectionId,
	pathname: string
): ContentMetadata | null {
	const href = pathname.replace(/\/+$/, '');
	if (href !== `/${sectionId}` && !href.startsWith(`/${sectionId}/`)) return null;
	const page = pagesByRoute.get(href);
	if (!page) return null;
	return {
		href,
		slug: page.slug,
		title: page.metadata.title ?? page.name,
		description: page.metadata.description,
		sourceType: page.sourceType
	};
}
export async function getContentSectionModule(
	sectionId: ContentSectionId,
	slug: string
): Promise<ContentModule | null> {
	const page = pagesByRoute.get(getContentSectionHref(sectionId, slug));
	if (!page) return null;
	return page.sourceType === 'svelte'
		? allSveltePages[`${prefix}${page.path}`]
		: await allMarkdownModules[`${prefix}${page.path}`]();
}
export function getContentSectionRawSource(
	sectionId: ContentSectionId,
	slug: string
): string | null {
	const page = pagesByRoute.get(getContentSectionHref(sectionId, slug));
	return page?.sourceType === 'markdown' ? (allMarkdownRaw[`${prefix}${page.path}`] ?? null) : null;
}
export function getContentSectionTocHeadings(
	sectionId: ContentSectionId,
	slug: string,
	selector: string
): ContentTocHeading[] {
	const source = getContentSectionRawSource(sectionId, slug);
	return source ? extractTocHeadings(parseContentSource(source).body, selector) : [];
}
export function getContentSectionAdjacentItems(sectionId: ContentSectionId, slug: string) {
	return getAdjacentItems(
		getContentSectionManifest(sectionId).filter((page) => !page.sidebarHidden),
		slug
	);
}
export function getContentSectionHref(sectionId: ContentSectionId, slug: string) {
	return getHref(`/${sectionId}`, slug);
}
export function getContentSectionRawHref(sectionId: ContentSectionId, slug: string) {
	return `/${sectionId}/raw/${slug || 'index'}`;
}
export function getContentSectionByPathname(pathname: string) {
	const normalized = pathname.replace(/\/+$/, '');
	return (
		contentSections.find(
			(view) => normalized === `/${view.id}` || normalized.startsWith(`/${view.id}/`)
		) ?? null
	);
}

function extractHeadingLevels(selector: string) {
	const levels = new Set<number>();
	const headingRe = /\bh([1-6])\b/gi;
	let match: RegExpExecArray | null;

	while ((match = headingRe.exec(selector))) {
		levels.add(Number(match[1]));
	}

	return levels.size > 0 ? levels : new Set([2, 3]);
}

function decodeHtmlEntities(value: string) {
	const namedEntities: Record<string, string> = {
		amp: '&',
		lt: '<',
		gt: '>',
		quot: '"',
		apos: "'"
	};

	return value.replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, (entity, raw: string) => {
		if (raw.startsWith('#')) {
			const radix = raw[1].toLowerCase() === 'x' ? 16 : 10;
			const codePoint = Number.parseInt(raw.slice(radix === 16 ? 2 : 1), radix);
			return Number.isFinite(codePoint) ? String.fromCodePoint(codePoint) : entity;
		}

		return namedEntities[raw.toLowerCase()] ?? entity;
	});
}

function normalizeHeadingText(rawText: string) {
	return decodeHtmlEntities(
		rawText
			.replace(/\s+#+\s*$/g, '')
			.replace(/\\([\\`*_[\]{}()#+.!|-])/g, '$1')
			.replace(/!\[([^\]]*)\]\([^)]+\)/g, '$1')
			.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
			.replace(/`([^`]*)`/g, '$1')
			.replace(/<[^>]+>/g, '')
			.replace(/\{([^{}]*)\}/g, '$1')
			.replace(/[*_~]/g, '')
			.replace(/\s+/g, ' ')
			.trim()
	);
}

function extractTocHeadings(source: string, selector: string): ContentTocHeading[] {
	const levels = extractHeadingLevels(selector);
	const slugger = new GithubSlugger();
	const headings: ContentTocHeading[] = [];
	let inFence = false;

	for (const line of source.split(/\r?\n/)) {
		if (/^\s*(```|~~~)/.test(line)) {
			inFence = !inFence;
			continue;
		}

		if (inFence) continue;

		const match = /^( {0,3})(#{1,6})\s+(.+?)\s*$/.exec(line);
		if (!match) continue;

		const level = match[2].length;
		if (!levels.has(level)) continue;

		const text = normalizeHeadingText(match[3]);
		if (!text) continue;

		headings.push({
			id: slugger.slug(text),
			text,
			level
		});
	}

	return headings;
}
