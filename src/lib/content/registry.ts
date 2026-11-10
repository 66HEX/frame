import { parseContentMetadata } from './frontmatter';
import type { ContentFrontmatter, ContentItem } from './types';

export type ContentSource = {
	/** Path relative to site/content, including the view folder and extension. */
	path: string;
	metadata: unknown;
};

export type RegisteredPage = ContentItem & {
	path: string;
	metadata: ContentFrontmatter;
	sourceType: 'markdown' | 'svelte';
};

type Directory = {
	name: string;
	path: string;
	index?: Entry;
	meta?: Entry;
	pages: Entry[];
	children: Map<string, Directory>;
};
type Entry = { path: string; stem: string; metadata: ContentFrontmatter };

const reservedViews = new Set([
	'og',
	'favicon.svg',
	'site.webmanifest',
	'llms.txt',
	'robots.txt',
	'sitemap.xml'
]);
const reservedPages = new Set(['og', 'raw', 'embed', 'index']);
const segmentPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const pathlessPattern = /^\([a-z0-9]+(?:-[a-z0-9]+)*\)$/;
const titleFromPath = (name: string) =>
	name
		.replace(/[()]/g, '')
		.replace(/-/g, ' ')
		.replace(/\b\w/g, (char) => char.toUpperCase());
const compare = (a: { order: number; name: string }, b: { order: number; name: string }) =>
	a.order - b.order || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
const directory = (name: string, path: string): Directory => ({
	name,
	path,
	pages: [],
	children: new Map()
});

/** Build routes and sidebar order from files. No component imports or hand-written page lists. */
export function buildContentRegistry(sources: ContentSource[]) {
	const roots = new Map<string, Directory>();
	for (const source of sources) {
		const segments = source.path.split('/');
		const id = segments.shift() ?? '';
		const filename = segments.pop() ?? '';
		if (!/^[a-z][a-z0-9-]*$/.test(id) || reservedViews.has(id)) {
			throw new Error(`Invalid or reserved view id in ${source.path}.`);
		}
		if (!/\.(svx|mdx|md|svelte)$/.test(filename))
			throw new Error(`Unsupported content file: ${source.path}`);
		let parent: Directory = roots.get(id) ?? directory(id, id);
		roots.set(id, parent);
		for (const segment of segments) {
			if (!segmentPattern.test(segment) && !pathlessPattern.test(segment))
				throw new Error(`Invalid folder in ${source.path}: ${segment}`);
			const child: Directory =
				parent.children.get(segment) ?? directory(segment, `${parent.path}/${segment}`);
			parent.children.set(segment, child);
			parent = child;
		}
		const stem = filename.replace(/\.[^.]+$/, '');
		if (!segmentPattern.test(stem) && stem !== '_meta')
			throw new Error(`Invalid page filename: ${source.path}`);
		const metadata = parseContentMetadata(source.metadata, source.path);
		const entry = { ...source, stem, metadata };
		if (metadata.view && (stem !== 'index' || segments.length))
			throw new Error(`view metadata belongs in the view's root index: ${source.path}`);
		if (metadata.group && stem !== 'index' && stem !== '_meta')
			throw new Error(`group metadata belongs in index or _meta: ${source.path}`);
		if (stem === '_meta') {
			if (filename.endsWith('.svelte'))
				throw new Error(`Use Markdown for folder metadata: ${source.path}`);
			if (Object.keys(metadata).some((key) => key !== 'group'))
				throw new Error(
					`Only group metadata is allowed in ${source.path}. Use index for a landing page.`
				);
			if (parent.meta)
				throw new Error(`Duplicate folder metadata: ${parent.meta.path}, ${source.path}`);
			parent.meta = entry;
		} else if (stem === 'index') {
			if (parent.index) throw new Error(`Duplicate page: ${parent.index.path}, ${source.path}`);
			parent.index = entry;
		} else parent.pages.push(entry);
	}
	if (!roots.size) throw new Error('Add at least one view with an index page under site/content.');
	const views = [...roots.values()]
		.map((root) => {
			if (!root.index) throw new Error(`View ${root.name} needs a root index page.`);
			const view = root.index.metadata.view ?? {};
			const label = view.label ?? titleFromPath(root.name);
			const pages: RegisteredPage[] = [];
			const routes = new Map<string, string>();
			const visit = (dir: Directory, url: string[], parentLabels: string[]) => {
				if (dir.meta && dir.index?.metadata.group)
					throw new Error(`Define group metadata only once in ${dir.path}: index or _meta.`);
				const group = dir.meta?.metadata.group ?? dir.index?.metadata.group;
				const groupLabel = group?.label ?? (dir === root ? label : titleFromPath(dir.name));
				const groupLabels = [...parentLabels, groupLabel];
				const add = (entry: Entry) => {
					const slug = [...url, ...(entry.stem === 'index' ? [] : [entry.stem])].join('/');
					if (reservedPages.has(slug.split('/')[0]))
						throw new Error(`Invalid or reserved page slug: ${root.name}/${slug} (${entry.path})`);
					if (routes.has(slug))
						throw new Error(
							`Duplicate route /${root.name}/${slug}: ${routes.get(slug) ?? ''}, ${entry.path}`
						);
					routes.set(slug, entry.path);
					const metadata = entry.metadata;
					pages.push({
						path: entry.path,
						slug,
						name:
							metadata.sidebar?.label ??
							metadata.title ??
							(entry.stem === 'index' ? groupLabel : titleFromPath(entry.stem)),
						category: groupLabels.join(' / '),
						categoryId: dir.path,
						sidebarHidden: metadata.sidebar?.hidden ?? false,
						showPagination: metadata.showPagination,
						metadata,
						sourceType: entry.path.endsWith('.svelte') ? 'svelte' : 'markdown'
					});
				};
				const entries = [...(dir.index ? [dir.index] : []), ...dir.pages].map((entry) => ({
					name: entry.stem,
					order: entry.metadata.order ?? (entry.stem === 'index' ? -1 : 0),
					entry
				}));
				const children = [...dir.children.values()].map((child) => ({
					name: child.name,
					order:
						child.meta?.metadata.group?.order ??
						child.index?.metadata.group?.order ??
						child.index?.metadata.order ??
						0,
					child
				}));
				// Keep a folder's pages together so sidebar order and pagination agree.
				for (const { entry } of entries.sort(compare)) add(entry);
				for (const { child } of children.sort(compare)) {
					visit(
						child,
						[...url, ...(pathlessPattern.test(child.name) ? [] : [child.name])],
						dir === root ? [] : groupLabels
					);
				}
			};
			visit(root, [], []);
			return {
				id: root.name,
				label,
				icon: view.icon ?? 'BookOpenIcon',
				layout: view.layout ?? 'article',
				ui: view.ui,
				order: view.order ?? 0,
				pages
			};
		})
		.sort((a, b) => compare({ order: a.order, name: a.id }, { order: b.order, name: b.id }));
	return views;
}
