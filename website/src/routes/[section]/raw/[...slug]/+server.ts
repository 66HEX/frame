import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import {
	getContentSectionByPathname,
	getContentSectionRawSource,
	getContentSectionManifest
} from '$lib/content/sections';
import { contentSections } from '$lib/content/sections';

const normalize = (value: string) => value.replace(/^\/+|\/+$/g, '');

export const prerender = true;

export const entries = () => {
	const result: { section: string; slug: string }[] = [];
	for (const section of contentSections) {
		const manifest = getContentSectionManifest(section.id);
		for (const item of manifest) {
			const slug = item.slug || 'index';
			if (getContentSectionRawSource(section.id, item.slug)) {
				result.push({ section: section.id, slug });
			}
		}
	}
	return result;
};

export const GET: RequestHandler = ({ params }) => {
	const sectionParam = normalize(params.section);
	const sectionPath = sectionParam ? `/${sectionParam}` : '/';
	const section = getContentSectionByPathname(sectionPath);
	if (!section) {
		error(404, 'Section not found');
	}

	const slugParam = normalize(params.slug);
	const targetSlug = slugParam === '' || slugParam === 'index' ? '' : slugParam;

	const content = getContentSectionRawSource(section.id, targetSlug);

	if (!content) {
		error(404, 'Document not found');
	}

	return new Response(content, {
		headers: {
			'Content-Type': 'text/markdown; charset=utf-8',
			'Cache-Control': 'public, max-age=60',
			'X-Robots-Tag': 'noindex'
		}
	});
};
