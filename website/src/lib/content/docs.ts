import { getContentSectionConfig, getContentSectionManifest } from './sections';
import {
	getContentSectionHref,
	getContentSectionMetadata,
	getContentSectionRawHref,
	getContentSectionRawSource
} from './sections';
import { parseContentSource } from './frontmatter';

export type DocsDocument = {
	title: string;
	description: string;
	section: string;
	slug: string;
	href: string;
	pathname: string;
	rawPath: string;
	content: string;
};
type DocsNavigationItem = Pick<DocsDocument, 'href' | 'pathname' | 'section' | 'slug' | 'title'>;
export type DocsNavigationGroup = {
	id: string;
	title: string;
	items: readonly DocsNavigationItem[];
};
export type DocsPaginationItem = Pick<DocsDocument, 'href' | 'title'>;

/** Resolve a single view's navigation, preserving categories at arbitrary nesting depth. */
export function getNavigationGroups(sectionId: string): DocsNavigationGroup[] {
	const section = getContentSectionConfig(sectionId);
	if (!section) return [];
	const groups = new Map<string, { id: string; title: string; items: DocsNavigationItem[] }>();
	for (const item of getContentSectionManifest(sectionId)) {
		if (item.sidebarHidden) continue;
		const category = item.category;
		const group = groups.get(item.categoryId) ?? {
			id: item.categoryId,
			title: category,
			items: []
		};
		group.items.push({
			title: item.name,
			slug: item.slug,
			section: category,
			href: getContentSectionHref(sectionId, item.slug),
			pathname: getContentSectionHref(sectionId, item.slug)
		});
		groups.set(item.categoryId, group);
	}
	return [...groups.values()];
}

export function getDocsDocument(sectionId: string, pathname: string): DocsDocument | null {
	const metadata = getContentSectionMetadata(sectionId, pathname);
	if (!metadata) return null;
	const navigation = getNavigationGroups(sectionId)
		.flatMap((group) => group.items)
		.find((item) => item.pathname === metadata.href);
	const source = getContentSectionRawSource(sectionId, metadata.slug);
	return {
		...metadata,
		description: metadata.description ?? '',
		pathname: metadata.href,
		section: navigation?.section ?? getContentSectionConfig(sectionId)?.label ?? sectionId,
		rawPath: getContentSectionRawHref(sectionId, metadata.slug),
		content: source ? parseContentSource(source).body : ''
	};
}
