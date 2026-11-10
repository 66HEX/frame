import type { RequestHandler } from './$types';
import { siteConfig } from '$lib/site/site';
import { contentSections } from '$lib/content/sections';
import { getContentSectionHref, getContentSectionManifest } from '$lib/content/sections';

type SitemapEntry = {
	path: string;
	changefreq?: string;
	priority?: string;
};

const staticPages: SitemapEntry[] = [
	{ path: '/', changefreq: 'weekly', priority: '1.0' },
	{ path: '/llms.txt', changefreq: 'weekly', priority: '0.4' }
];

const buildTimestamp = new Date().toISOString();

const toAbsoluteUrl = (origin: string, path: string) => new URL(path, origin).href;

const createUrlEntry = (origin: string, entry: SitemapEntry) => {
	const loc = toAbsoluteUrl(origin, entry.path);
	const changefreqTag = entry.changefreq ? `<changefreq>${entry.changefreq}</changefreq>` : '';
	const priorityTag = entry.priority ? `<priority>${entry.priority}</priority>` : '';

	return `<url><loc>${loc}</loc><lastmod>${buildTimestamp}</lastmod>${changefreqTag}${priorityTag}</url>`;
};

const dedupeEntries = (entries: SitemapEntry[]) => {
	const map = new Map<string, SitemapEntry>();
	for (const entry of entries) {
		if (!map.has(entry.path)) {
			map.set(entry.path, entry);
		}
	}
	return Array.from(map.values());
};

export const GET: RequestHandler = () => {
	const canonicalOrigin = new URL(siteConfig.url).origin;
	const sectionRootEntries: SitemapEntry[] = contentSections.map((section) => ({
		path: `/${section.id}`,
		changefreq: 'weekly',
		priority: '0.9'
	}));
	const sectionEntries: SitemapEntry[] = contentSections.flatMap((section) =>
		getContentSectionManifest(section.id).map((item) => ({
			path: getContentSectionHref(section.id, item.slug),
			changefreq: 'weekly',
			priority: '0.8'
		}))
	);

	const uniqueEntries = dedupeEntries([...staticPages, ...sectionRootEntries, ...sectionEntries]);
	const body =
		`<?xml version="1.0" encoding="UTF-8"?>` +
		`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">` +
		uniqueEntries.map((entry) => createUrlEntry(canonicalOrigin, entry)).join('') +
		`</urlset>`;

	return new Response(body, {
		headers: {
			'content-type': 'application/xml',
			'cache-control': 'public, max-age=3600'
		}
	});
};
