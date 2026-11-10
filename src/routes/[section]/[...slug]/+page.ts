import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getContentSectionModule, getContentSectionManifest } from '$lib/content/sections';
import { contentSections } from '$lib/content/sections';

export const prerender = true;

export const entries = () => {
	const result: { section: string; slug: string }[] = [];

	for (const section of contentSections) {
		const manifest = getContentSectionManifest(section.id);
		for (const item of manifest) {
			result.push({
				section: section.id,
				slug: item.slug
			});
		}
	}

	return result;
};

export const load: PageLoad = async ({ params }) => {
	const { section: sectionId, slug } = params;

	const mod = await getContentSectionModule(sectionId, slug);
	if (!mod) {
		error(404, 'Page not found');
	}

	return {
		component: mod.default,
		slug
	};
};
