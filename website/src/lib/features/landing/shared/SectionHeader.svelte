<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ArrowRightIcon } from '$lib/icons';
	import SectionTitle from './SectionTitle.svelte';
	import SectionDescription from './SectionDescription.svelte';
	import TextLink from './TextLink.svelte';

	let {
		id,
		title,
		description,
		href,
		linkLabel
	}: {
		id: string;
		title: Snippet;
		description: string;
		href?: string;
		linkLabel?: string;
	} = $props();
</script>

<div class="grid items-start gap-8 pb-12 md:grid-cols-2 md:gap-16 md:pb-20">
	<div><SectionTitle {id}>{@render title()}</SectionTitle></div>
	<div>
		<SectionDescription>{description}</SectionDescription>
		{#if href && linkLabel}
			<TextLink {href} class="mt-4">{linkLabel}<ArrowRightIcon class="size-4" /></TextLink>
		{/if}
	</div>
</div>
