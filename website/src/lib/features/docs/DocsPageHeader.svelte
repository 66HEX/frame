<script lang="ts">
	import type { DocsDocument } from '$lib/content/docs';
	import type { SectionUiConfig } from '$lib/site/content-ui';
	import DocsActions from './DocsActions.svelte';

	type Props = {
		doc: DocsDocument;
		actions: SectionUiConfig['pageActions'];
	};

	let { doc, actions }: Props = $props();

</script>

<header class="mb-10 space-y-6 border-b border-border pb-8">
	<div class="space-y-4">
		<div class="space-y-3">
			<p class="flex items-center gap-1.5 text-small font-medium text-muted-foreground">
				<span>{doc.section}</span>
			</p>
			<h1
				class="font-heading text-title-4 font-medium text-balance sm:text-title-5"
			>
				{doc.title}
			</h1>
			<p class="max-w-2xl text-regular text-muted-foreground">{doc.description}</p>
		</div>

	</div>

	{#if actions.enabled}<DocsActions
		config={actions}
		rawPath={doc.rawPath}
	/>{/if}
</header>
