import type { RequestHandler } from './$types';
import { siteConfig } from '$lib/site/site';
import { contentSections } from '$lib/content/sections';
import {
	getContentSectionRawSource,
	getContentSectionHref,
	getContentSectionManifest,
	getContentSectionMetadata,
	getContentSectionRawHref,
	type ContentSectionId
} from '$lib/content/sections';

type ContentEntry = {
	sectionId: ContentSectionId;
	sectionLabel: string;
	slug: string;
	fallbackTitle: string;
};

const summary = siteConfig.description;

const detailParagraphs = [
	'LLM-friendly Markdown for every page is available at `/<section>/raw/<slug>`; this is the source content without navigation chrome.',
	'Use `/sitemap.xml` for URL discovery and `/robots.txt` for crawl guidance.'
];

const buildContentEntry = (origin: string, entry: ContentEntry) => {
	const pagePath = getContentSectionHref(entry.sectionId, entry.slug);
	const metadata = getContentSectionMetadata(entry.sectionId, pagePath);
	const title = metadata?.title ?? entry.fallbackTitle;
	const description = metadata?.description ?? `${entry.sectionLabel} page for ${title}.`;
	const rawPath = getContentSectionRawHref(entry.sectionId, entry.slug);
	const link = new URL(rawPath, origin).href;
	return `- [${title}](${link}): ${description}`;
};

const dedupeEntries = (entries: ContentEntry[]) => {
	const map = new Map<string, ContentEntry>();
	for (const entry of entries) {
		const key = `${entry.sectionId}:${entry.slug}`;
		if (!map.has(key)) {
			map.set(key, entry);
		}
	}
	return Array.from(map.values());
};

const buildSection = (title: string, items: string[]) => {
	if (items.length === 0) return [];
	return [`## ${title}`, '', ...items];
};

export const GET: RequestHandler = () => {
	const canonicalOrigin = new URL(siteConfig.url).origin;
	const optionalLinks = [
		`- [GitHub](${siteConfig.links.github}): Source code, issues, and discussions.`,
		`- [Releases](${siteConfig.links.github}/releases/latest): Download the latest Frame desktop application.`
	];

	const sectionBlocks = contentSections.flatMap((section) => {
		const entries = dedupeEntries(
			getContentSectionManifest(section.id)
				.filter((item) => getContentSectionRawSource(section.id, item.slug) !== null)
				.map((item) => ({
					sectionId: section.id,
					sectionLabel: section.label,
					slug: item.slug,
					fallbackTitle: item.name
				}))
		);

		return buildSection(
			section.label,
			entries.map((entry) => buildContentEntry(canonicalOrigin, entry))
		);
	});

	const lines = [
		`# ${siteConfig.name}`,
		'',
		`> ${summary}`,
		'',
		...detailParagraphs,
		'',
		...sectionBlocks,
		'',
		...buildSection('Optional', optionalLinks),
		''
	];

	const body =
		lines
			.join('\n')
			.replace(/\n{3,}/g, '\n\n')
			.trim() + '\n';

	return new Response(body, {
		headers: {
			'content-type': 'text/plain; charset=utf-8',
			'cache-control': 'public, max-age=3600'
		}
	});
};
