import { fileURLToPath, URL } from 'node:url';
import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { escapeSvelte, mdsvex } from 'mdsvex';
import { createHighlighter } from 'shiki';
import rehypeSlug from 'rehype-slug';
import { type Config } from '@sveltejs/kit/vite';
import type { Root, Element, Text, ElementContent } from 'hast';
import type { Node } from 'unist';

const tableCellFormatter = () => {
	return (tree: Root): void => {
		const ancestors: Element[] = [];

		const visit = (node: Node, parent: Root | Element | null = null, index = 0): void => {
			const isElement = node.type === 'element';
			const isRoot = node.type === 'root';

			if (isElement) {
				ancestors.push(node as Element);
			}

			if (node.type === 'text') {
				const textNode = node as Text;
				if (typeof textNode.value === 'string' && textNode.value.includes('\\|')) {
					const directParent = ancestors[ancestors.length - 1];
					const grandParent = ancestors[ancestors.length - 2];
					const isCodeBlock = directParent.tagName === 'code' && grandParent.tagName === 'pre';

					if (!isCodeBlock) {
						textNode.value = textNode.value.replace(/\\\|/g, '|');
					}
				}
			}

			if (isElement) {
				const el = node as Element;
				if (
					el.tagName === 'code' &&
					Array.isArray(el.children) &&
					el.children.length === 1 &&
					el.children[0].type === 'text'
				) {
					const parentNode = ancestors[ancestors.length - 2];
					const isBlockCode = parentNode.tagName === 'pre';
					const insideTableCell = ancestors.some((ancestor) => {
						if (ancestor === el) return false;
						const a = ancestor;
						return a.tagName === 'td' || a.tagName === 'th';
					});

					const childText = el.children[0];
					let raw = typeof childText.value === 'string' ? childText.value : '';
					if (raw.includes('\\|')) {
						raw = raw.replace(/\\\|/g, '|');
						childText.value = raw;
					}

					if (!isBlockCode && insideTableCell && raw.includes('|') && parent) {
						const parentChildren = parent.children;
						if (Array.isArray(parentChildren)) {
							const segments = raw.split('|').map((segment: string) => segment.trim());
							if (segments.length > 1) {
								const replacements: ElementContent[] = segments.flatMap(
									(segment: string, segmentIndex: number) => {
										const codeNode: Element = {
											type: 'element',
											tagName: 'code',
											properties: el.properties,
											children: [
												{
													type: 'text',
													value: segment
												}
											]
										};

										if (segmentIndex === segments.length - 1) {
											return [codeNode];
										}

										return [codeNode, { type: 'text', value: ' ' }];
									}
								);

								parentChildren.splice(index, 1, ...replacements);
								ancestors.pop();
								replacements.forEach((child: Node, childIndex: number) => {
									visit(child, parent, index + childIndex);
								});
								return;
							}
						}
					}
				}
			}

			const childNodes = isElement || isRoot ? (node as Root | Element).children : [];
			for (let i = 0; i < childNodes.length; i += 1) {
				visit(childNodes[i], node as Root | Element, i);
			}

			if (isElement) {
				ancestors.pop();
			}
		};

		visit(tree);
	};
};

const themes = {
	light: 'github-light',
	dark: 'github-dark'
};
const highlighter = await createHighlighter({
	themes: Object.values(themes),
	langs: ['svelte', 'bash', 'json', 'typescript', 'tsx', 'vue', 'wgsl', 'css', 'html']
});

const markdownLayout = fileURLToPath(
	new URL('./src/lib/features/docs/MarkdownLayout.svelte', import.meta.url)
);

const config: Config = {
	extensions: ['.svelte', '.svx', '.mdx', '.md'],
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: [
		mdsvex({
			extensions: ['.svx', '.mdx', '.md'],
			layout: {
				_: markdownLayout
			},
			rehypePlugins: [tableCellFormatter, rehypeSlug],
			highlight: {
				highlighter: (code: string, lang: string | null = 'text') => {
					const safeLang = lang ?? 'text';
					const lightHtml = escapeSvelte(
						highlighter.codeToHtml(code, {
							lang: safeLang,
							theme: themes.light,
							tabindex: false
						})
					);
					const darkHtml = escapeSvelte(
						highlighter.codeToHtml(code, {
							lang: safeLang,
							theme: themes.dark,
							tabindex: false
						})
					);
					const htmlLightProp = JSON.stringify(lightHtml);
					const htmlDarkProp = JSON.stringify(darkHtml);
					const langProp = JSON.stringify(lang);
					const rawProp = JSON.stringify(code);
					return `<svelte:component this={Reflect.get(globalThis, "__DocsMarkdownPre")} lang={${langProp}} htmlLight={${htmlLightProp}} htmlDark={${htmlDarkProp}} raw={${rawProp}} />`;
				}
			}
		}),
		vitePreprocess()
	],

	adapter: adapter(),
	alias: { $lib: "./src/lib" }
};

export default config;
