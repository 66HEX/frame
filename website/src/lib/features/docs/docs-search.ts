import GithubSlugger from 'github-slugger';
import type { DocsDocument } from '$lib/content/docs';

type DocsSearchMatchType = 'page' | 'heading' | 'content';

type SearchableDocument = Pick<
	DocsDocument,
	'content' | 'description' | 'href' | 'section' | 'title'
>;

type IndexedDocsSearchEntry = {
	anchor?: string;
	content?: string;
	heading?: string;
	href: string;
	id: string;
	matchType: DocsSearchMatchType;
	order: number;
	section: string;
	title: string;
};

export type DocsSearchIndex = readonly IndexedDocsSearchEntry[];

export type DocsSearchResult = Omit<IndexedDocsSearchEntry, 'content' | 'order'> & {
	score: number;
	snippet?: string;
};

export type DocsSearchGroup = {
	children: readonly DocsSearchResult[];
	page: DocsSearchResult;
	score: number;
};

export type DocsSearchOptions = {
	maxChildrenPerGroup?: number;
	maxGroups?: number;
};

export type DocsSearchHighlight = {
	highlighted: boolean;
	text: string;
};

const defaultOptions = {
	maxChildrenPerGroup: 3,
	maxGroups: 6
} as const satisfies Required<DocsSearchOptions>;

function removeNonContentBlocks(content: string) {
	return content
		.replace(/<!--[\s\S]*?-->/g, ' ')
		.replace(/<script\b[\s\S]*?<\/script\b[^>]*>/gi, ' ')
		.replace(/<style\b[\s\S]*?<\/style\b[^>]*>/gi, ' ')
		.replace(/```[\s\S]*?```/g, ' ');
}

function toPlainText(content: string) {
	return content
		.replace(/!\[([^\]]*)\]\([^)]+\)/g, '$1')
		.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
		.replace(/<[^>]+>/g, ' ')
		.replace(/`([^`]+)`/g, '$1')
		.replace(/^\s*>\s?/gm, '')
		.replace(/^\s*(?:[-+*]|\d+\.)\s+/gm, '')
		.replace(/[*_~]/g, '')
		.replace(/\s+/g, ' ')
		.trim();
}

function snippetFor(content: string, query: string, maxLength = 132) {
	const normalizedContent = content.toLocaleLowerCase('en');
	const normalizedQuery = query.toLocaleLowerCase('en');
	const matchIndex = normalizedContent.indexOf(normalizedQuery);

	if (content.length <= maxLength) return content;
	if (matchIndex === -1) return `${content.slice(0, maxLength).trimEnd()}…`;

	const contextLength = Math.max(0, maxLength - query.length);
	const start = Math.max(0, matchIndex - Math.floor(contextLength / 2));
	const end = Math.min(content.length, matchIndex + query.length + Math.ceil(contextLength / 2));
	const snippet = content.slice(start, end).trim();

	return `${start > 0 ? '…' : ''}${snippet}${end < content.length ? '…' : ''}`;
}

export function buildDocsSearchIndex(
	documents: readonly SearchableDocument[]
): readonly IndexedDocsSearchEntry[] {
	const index: IndexedDocsSearchEntry[] = [];
	let entryOrder = 0;

	for (const document of documents) {
		index.push({
			href: document.href,
			id: `${document.href}::page`,
			matchType: 'page',
			order: entryOrder++,
			section: document.section,
			title: document.title
		});

		if (document.description) {
			index.push({
				anchor: '',
				content: document.description,
				href: document.href,
				id: `${document.href}::description`,
				matchType: 'content',
				order: entryOrder++,
				section: document.section,
				title: document.title
			});
		}

		const slugger = new GithubSlugger();
		const lines = removeNonContentBlocks(document.content).split('\n');
		let currentAnchor = '';
		let currentHeading: string | undefined;
		let contentBuffer: string[] = [];
		let contentIndex = 0;

		const flushContent = () => {
			const content = toPlainText(contentBuffer.join(' '));
			contentBuffer = [];

			if (content.length <= 10) return;

			index.push({
				anchor: currentAnchor,
				content,
				href: `${document.href}${currentAnchor}`,
				id: `${document.href}${currentAnchor}::content:${contentIndex++}`,
				matchType: 'content',
				order: entryOrder++,
				section: document.section,
				title: document.title,
				...(currentHeading ? { heading: currentHeading } : {})
			});
		};

		for (const line of lines) {
			const headingMatch = /^(#{2,4})\s+(.+?)\s*#*\s*$/.exec(line);

			if (!headingMatch) {
				if (line.trim()) contentBuffer.push(line);
				continue;
			}

			flushContent();
			const heading = toPlainText(headingMatch[2] ?? '');
			if (!heading) continue;

			currentHeading = heading;
			currentAnchor = `#${slugger.slug(heading)}`;
			index.push({
				anchor: currentAnchor,
				heading,
				href: `${document.href}${currentAnchor}`,
				id: `${document.href}${currentAnchor}::heading`,
				matchType: 'heading',
				order: entryOrder++,
				section: document.section,
				title: document.title
			});
		}

		flushContent();
	}

	return index;
}

function scoreEntry(entry: IndexedDocsSearchEntry, query: string) {
	if (entry.matchType === 'page') {
		const title = entry.title.toLocaleLowerCase('en');
		if (!title.includes(query)) return 0;
		return title.startsWith(query) ? 15 : 10;
	}

	if (entry.matchType === 'heading') {
		return entry.heading?.toLocaleLowerCase('en').includes(query) ? 5 : 0;
	}

	return entry.content?.toLocaleLowerCase('en').includes(query) ? 1 : 0;
}

function toSearchResult(entry: IndexedDocsSearchEntry, score: number, query: string) {
	return {
		href: entry.href,
		id: entry.id,
		matchType: entry.matchType,
		section: entry.section,
		title: entry.title,
		score,
		...(entry.anchor ? { anchor: entry.anchor } : {}),
		...(entry.heading ? { heading: entry.heading } : {}),
		...(entry.content ? { snippet: snippetFor(entry.content, query) } : {})
	} satisfies DocsSearchResult;
}

export function searchDocsIndex(
	index: readonly IndexedDocsSearchEntry[],
	query: string,
	options: DocsSearchOptions = {}
): readonly DocsSearchGroup[] {
	const normalizedQuery = query.trim().toLocaleLowerCase('en');
	if (!normalizedQuery) return [];

	const pageByHref = new Map(
		index.filter((entry) => entry.matchType === 'page').map((entry) => [entry.href, entry])
	);
	const groups = new Map<
		string,
		{
			children: { order: number; result: DocsSearchResult }[];
			page: DocsSearchResult;
			score: number;
		}
	>();

	for (const entry of index) {
		const score = scoreEntry(entry, normalizedQuery);
		if (score === 0) continue;

		const pageHref = entry.matchType === 'page' ? entry.href : entry.href.split('#', 1)[0]!;
		const indexedPage = pageByHref.get(pageHref);
		if (!indexedPage) continue;

		let group = groups.get(pageHref);
		if (!group) {
			group = {
				children: [],
				page: toSearchResult(indexedPage, 0, query.trim()),
				score: 0
			};
			groups.set(pageHref, group);
		}

		const result = toSearchResult(entry, score, query.trim());

		if (entry.matchType === 'page') group.page = result;
		else group.children.push({ order: entry.order, result });
		group.score = Math.max(group.score, score);
	}

	const maxGroups = options.maxGroups ?? defaultOptions.maxGroups;
	const maxChildrenPerGroup = options.maxChildrenPerGroup ?? defaultOptions.maxChildrenPerGroup;

	return [...groups.values()]
		.sort((left, right) => {
			const scoreDifference = right.score - left.score;
			if (scoreDifference !== 0) return scoreDifference;
			return left.page.title.localeCompare(right.page.title);
		})
		.slice(0, Math.max(0, maxGroups))
		.map((group) => ({
			page: group.page,
			score: group.score,
			children: group.children
				.sort((left, right) => {
					const scoreDifference = right.result.score - left.result.score;
					if (scoreDifference !== 0) return scoreDifference;
					return left.order - right.order;
				})
				.slice(0, Math.max(0, maxChildrenPerGroup))
				.map(({ result }) => result)
		}));
}

export function highlightDocsSearchMatch(
	text: string,
	query: string
): readonly DocsSearchHighlight[] {
	const normalizedQuery = query.trim();
	if (!normalizedQuery) return [{ highlighted: false, text }];

	const escapedQuery = normalizedQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const parts = text.split(new RegExp(`(${escapedQuery})`, 'gi'));

	return parts.filter(Boolean).map((part) => ({
		highlighted: part.toLocaleLowerCase('en') === normalizedQuery.toLocaleLowerCase('en'),
		text: part
	}));
}
