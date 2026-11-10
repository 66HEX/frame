<script module lang="ts">
	import MarkdownPre from './markdown/MarkdownPre.svelte';

	Reflect.set(globalThis, '__DocsMarkdownPre', MarkdownPre);

	export { default as a } from './markdown/Link.svelte';
	export { default as blockquote } from './markdown/Blockquote.svelte';
	export { default as code } from './markdown/Code.svelte';
	export { default as h1 } from './markdown/H1.svelte';
	export { default as h2 } from './markdown/H2.svelte';
	export { default as h3 } from './markdown/H3.svelte';
	export { default as h4 } from './markdown/H4.svelte';
	export { default as hr } from './markdown/Divider.svelte';
	export { default as li } from './markdown/ListItem.svelte';
	export { default as ol } from './markdown/OrderedList.svelte';
	export { default as p } from './markdown/Paragraph.svelte';
	export { default as strong } from './markdown/Strong.svelte';
	export { default as table } from './markdown/Table.svelte';
	export { default as tbody } from './markdown/Tbody.svelte';
	export { default as td } from './markdown/Td.svelte';
	export { default as th } from './markdown/Th.svelte';
	export { default as thead } from './markdown/Thead.svelte';
	export { default as tr } from './markdown/Tr.svelte';
	export { default as ul } from './markdown/UnorderedList.svelte';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<article data-doc-content class="w-full">
	{@render children()}
</article>
