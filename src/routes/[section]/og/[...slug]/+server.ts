import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { createOgImage } from '$lib/seo/og-image';
import {
	getContentSectionMetadata,
	getContentSectionByPathname,
	getContentSectionManifest
} from '$lib/content/sections';
import { contentSections } from '$lib/content/sections';
import { siteConfig } from '$lib/site/site';

export const prerender = true;

export const entries = () => {
	const result: { section: string; slug: string }[] = [];
	for (const section of contentSections) {
		const manifest = getContentSectionManifest(section.id);
		for (const item of manifest) {
			result.push({
				section: section.id,
				slug: item.slug || 'index'
			});
		}
	}
	return result;
};

const MAX_TITLE_LENGTH = 88;
const MAX_DESCRIPTION_LENGTH = 180;

const clampText = (value: string, maxLength: number) => {
	const text = value.trim();
	if (text.length <= maxLength) return text;
	return `${text.slice(0, maxLength - 1).trimEnd()}…`;
};

const getTitleFontSize = (title: string) => {
	if (title.length > 70) return 48;
	if (title.length > 48) return 56;
	return 64;
};

export const GET: RequestHandler = ({ params }) => {
	const sectionParam = params.section;
	const section = getContentSectionByPathname(`/${sectionParam}`);
	if (!section) error(404, 'Section not found');

	const rawSlug = params.slug.replace(/^\/+|\/+$/g, '');
	const slug = rawSlug === '' || rawSlug === 'index' ? '' : rawSlug;
	const metadata = getContentSectionMetadata(section.id, `/${section.id}/${slug}`);
	if (!metadata) error(404, 'Document not found');

	const title = clampText(metadata.title, MAX_TITLE_LENGTH);
	const description = clampText(
		metadata.description ?? siteConfig.description,
		MAX_DESCRIPTION_LENGTH
	);

	return createOgImage({
		title,
		description,
		titleFontSize: getTitleFontSize(title)
	});
};
