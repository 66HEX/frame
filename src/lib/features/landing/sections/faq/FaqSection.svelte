<script lang="ts">
	import * as Accordion from '$lib/components/ui/accordion';
	import { faqItems } from '../../data/content';
	import Section from '../../shared/Section.svelte';
	import SectionTitle from '../../shared/SectionTitle.svelte';
	import SectionDescription from '../../shared/SectionDescription.svelte';
</script>

<Section id="faq" aria-labelledby="faq-title" divided class="grid gap-10 md:grid-cols-2 md:gap-16">
	<div>
		<SectionTitle id="faq-title">Good questions.<br />Straight answers.</SectionTitle>
		<SectionDescription class="mt-6 max-w-sm"
			>Find practical guides and help with your first conversion in the <a
				class="text-foreground underline underline-offset-4"
				href="/docs">documentation</a
			>.</SectionDescription
		>
	</div>
	<Accordion.Root type="multiple" class="min-w-0">
		{#each faqItems as item (item.question)}
			<Accordion.Item value={item.question}>
				<Accordion.Trigger>{item.question}</Accordion.Trigger>
				<Accordion.Content><p>{item.answer}</p></Accordion.Content>
			</Accordion.Item>
		{/each}
	</Accordion.Root>
</Section>
