import type { IconComponent } from '$lib/icons';
import type { DeepPartial, SectionUiConfig } from '$lib/site/content-ui';

export type ContentData =
	| null
	| boolean
	| number
	| string
	| ContentData[]
	| { [key: string]: ContentData };

/** Shared by Markdown frontmatter and a Svelte page's module-level metadata export. */
export type ContentFrontmatter = {
	title?: string;
	description?: string;
	order?: number;
	sidebar?: { label?: string; hidden?: boolean };
	showPagination?: boolean;
	group?: { label?: string; order?: number };
	view?: {
		label?: string;
		icon?: string;
		order?: number;
		layout?: 'article' | 'workspace';
		ui?: DeepPartial<SectionUiConfig>;
	};
	data?: Record<string, ContentData>;
};

export type ContentSectionConfig = {
	id: string;
	label: string;
	icon: IconComponent;
	layout: 'article' | 'workspace';
	ui?: DeepPartial<SectionUiConfig>;
};

export type ContentItem = {
	slug: string;
	name: string;
	category: string;
	categoryId: string;
	sidebarHidden: boolean;
	showPagination?: boolean;
};
