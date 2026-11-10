import { getNavigationGroups, getDocsDocument } from '$lib/content/docs';
import { getContentSectionRawSource } from '$lib/content/sections';
import { buildDocsSearchIndex, type DocsSearchIndex } from './docs-search';

const indexes = new Map<string, DocsSearchIndex>();
export function getSearchIndex(sectionId: string): DocsSearchIndex {
	const cached = indexes.get(sectionId);
	if (cached) return cached;
	const index = buildDocsSearchIndex(
		getNavigationGroups(sectionId)
			.flatMap((group) => group.items)
			.flatMap((item) => {
				if (!getContentSectionRawSource(sectionId, item.slug)) return [];
				const document = getDocsDocument(sectionId, item.pathname);
				return document ? [document] : [];
			})
	);
	indexes.set(sectionId, index);
	return index;
}
