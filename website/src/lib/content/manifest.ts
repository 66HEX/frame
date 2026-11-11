import type { ContentItem } from './types';

export function getAdjacentItems(items: ContentItem[], slug: string) {
	const index = items.findIndex((item) => item.slug === slug);
	if (index === -1) {
		return { previous: null, next: null };
	}

	const previous = index > 0 ? items[index - 1] : null;
	const next = index < items.length - 1 ? items[index + 1] : null;
	return { previous, next };
}

export function getHref(basePath: string, slug: string) {
	return slug ? `${basePath}/${slug}` : basePath;
}
