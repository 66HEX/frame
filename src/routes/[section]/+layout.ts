import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import {
	getContentSectionAdjacentItems,
	getContentSectionMetadata,
	getContentSectionSlug,
	getContentSectionTocHeadings,
	getContentSectionUiConfig
} from '$lib/content/sections';
import { contentSections } from '$lib/content/sections';
import { resolveTocSelector } from '$lib/site/content-ui';

export const prerender = true;

export const load: LayoutLoad = ({ params, url }) => {
	const sectionId = params.section;

	const isKnownSection = contentSections.some((s) => s.id === sectionId);
	if (!isKnownSection) {
		error(404, 'Section not found');
	}

	const slug = getContentSectionSlug(sectionId, url.pathname);
	const { previous, next } = getContentSectionAdjacentItems(sectionId, slug);
	const metadata = getContentSectionMetadata(sectionId, url.pathname);
	const sectionUi = getContentSectionUiConfig(sectionId);
	const tocSelector = resolveTocSelector(sectionUi.toc, slug);
	const tocHeadings = getContentSectionTocHeadings(sectionId, slug, tocSelector);

	return {
		sectionId,
		slug,
		metadata,
		tocHeadings,
		previousDoc: previous,
		nextDoc: next
	};
};
