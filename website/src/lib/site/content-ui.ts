export type SectionUiConfig = {
	search: {
		enabled: boolean;
		label: string;
		placeholder: string;
		description: string;
		emptyHint: string;
		loading: string;
		error: string;
		retry: string;
		noResults: string;
		results: string;
		maxGroups: number;
		maxChildrenPerGroup: number;
	};
	sidebar: { navigationLabel: string; showRepositoryLink: boolean; repositoryLabel: string };
	pageActions: {
		enabled: boolean;
		showCopyMarkdown: boolean;
		showSourceLinks: boolean;
		label: string;
		moreLabel: string;
		newTabHint: string;
		copy: {
			idle: string;
			copying: string;
			success: string;
			error: string;
			announcement: string;
			errorAnnouncement: string;
		};
	};
	pagination: { enabled: boolean; label: string; previous: string; next: string };
	toc: {
		title: string;
		emptyLabel: string;
		defaultSelector: string;
		selectorOverrides: {
			slugPrefix: string;
			selector: string;
		}[];
	};
};

export type DeepPartial<T> = {
	[K in keyof T]?: T[K] extends (infer U)[]
		? DeepPartial<U>[]
		: T[K] extends object
			? DeepPartial<T[K]>
			: T[K];
};

/**
 * Global, strongly-typed settings for interactive content UI elements.
 * Adjust defaults here to tune behavior across the entire site experience.
 */
type ContentUiConfig = {
	shell: {
		viewSwitcherLabel: string;
		skipLink: string;
		homeLabel: string;
		showSidebar: string;
		hideSidebar: string;
	};
	theme: {
		storageKey: string;
		defaultMode: 'light' | 'dark' | 'system';
		showToggle: boolean;
		lightLabel: string;
		darkLabel: string;
	};
};

/**
 * Default section-level UI configuration shared across content sections.
 */
export const sectionUiDefaults: SectionUiConfig = {
	search: {
		enabled: true,
		label: 'Search documentation',
		placeholder: 'Search documentation…',
		description: 'Search documentation by page title, section heading, or page content.',
		emptyHint: 'Search by page title, section heading, or page content.',
		loading: 'Loading documentation search…',
		error: 'Unable to load search. Check your connection and try again.',
		retry: 'Retry search',
		noResults: 'No results for “{query}”. Try another search.',
		results: '{count} results available.',
		maxGroups: 8,
		maxChildrenPerGroup: 3
	},
	sidebar: {
		navigationLabel: 'Page navigation',
		showRepositoryLink: true,
		repositoryLabel: 'GitHub'
	},
	pageActions: {
		enabled: true,
		showCopyMarkdown: true,
		showSourceLinks: true,
		label: 'Documentation actions',
		moreLabel: 'More documentation actions',
		newTabHint: '(opens in a new tab)',
		copy: {
			idle: 'Copy Markdown',
			copying: 'Copying…',
			success: 'Markdown copied',
			error: 'Retry copy',
			announcement: 'Markdown copied.',
			errorAnnouncement: 'Unable to copy Markdown. Try again.'
		}
	},
	pagination: {
		enabled: true,
		label: 'Documentation pagination',
		previous: 'Previous',
		next: 'Next'
	},
	toc: {
		title: 'On this page',
		emptyLabel: 'No headings',
		defaultSelector: '[data-doc-content] > h2, [data-doc-content] > h3',
		selectorOverrides: []
	}
};

/**
 * Centralized UI defaults used across content sections and shared layout helpers.
 */
export const contentUiDefaults: ContentUiConfig = {
	shell: {
		viewSwitcherLabel: 'Select view',
		skipLink: 'Skip to content',
		homeLabel: 'Home',
		showSidebar: 'Show sidebar',
		hideSidebar: 'Hide sidebar'
	},
	theme: {
		storageKey: 'frame-theme',
		defaultMode: 'system',
		showToggle: true,
		lightLabel: 'Switch to light theme',
		darkLabel: 'Switch to dark theme'
	}
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
	typeof value === 'object' && value !== null && !Array.isArray(value);

const mergeDeep = <T>(base: T, overrides: DeepPartial<T>): T => {
	if (!isPlainObject(base) || !isPlainObject(overrides)) {
		return overrides as T;
	}

	const result: Record<string, unknown> = { ...base };
	for (const [key, value] of Object.entries(overrides)) {
		if (value === undefined) continue;
		const baseValue = (base as Record<string, unknown>)[key];
		if (Array.isArray(baseValue) && Array.isArray(value)) {
			result[key] = value;
			continue;
		}
		if (isPlainObject(baseValue) && isPlainObject(value)) {
			result[key] = mergeDeep(baseValue, value as DeepPartial<typeof baseValue>);
			continue;
		}
		result[key] = value;
	}

	return result as T;
};

export function mergeSectionUiConfig(
	overrides: DeepPartial<SectionUiConfig> = {}
): SectionUiConfig {
	return mergeDeep(sectionUiDefaults, overrides);
}

/**
 * Resolves the heading selector for table-of-contents generation based on a slug.
 *
 * @param tocConfig Section toc configuration.
 * @param slug Relative content slug (for example `changelog/1.0.0`).
 * @returns CSS selector used to extract headings from page content.
 */
export function resolveTocSelector(tocConfig: SectionUiConfig['toc'], slug?: string | null) {
	const normalizedSlug = slug ?? '';
	const override = tocConfig.selectorOverrides.find((item) =>
		normalizedSlug.startsWith(item.slugPrefix)
	);
	return override?.selector ?? tocConfig.defaultSelector;
}

/** Text templates are plain text, never HTML. */
export function formatUiText(template: string, values: Record<string, string | number>) {
	return template.replace(/\{(\w+)\}/g, (match, key: string) => String(values[key] ?? match));
}
