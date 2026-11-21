<script lang="ts">
	import * as Accordion from '$lib/components/ui/accordion';
	import { faqItems } from '../../data/content';
	import Section from '../../shared/Section.svelte';
	import SectionTitle from '../../shared/SectionTitle.svelte';
	import SectionDescription from '../../shared/SectionDescription.svelte';
</script>

<Section id="faq" aria-labelledby="faq-title" divided>
	<div class="mx-auto max-w-2xl text-center">
		<SectionTitle id="faq-title">Good questions.<br />Straight answers.</SectionTitle>
		<SectionDescription class="mt-4">
			Find practical guides and help with your first conversion in the
			<a class="text-foreground underline underline-offset-4" href="/docs">documentation</a>.
		</SectionDescription>
	</div>
	<div class="mx-auto mt-12 max-w-3xl md:mt-16">
		<Accordion.Root type="multiple">
			{#each faqItems as item (item.question)}
				<Accordion.Item value={item.question} class="py-1.5">
					<Accordion.Trigger>
						<span class="text-regular font-medium">{item.question}</span>
					</Accordion.Trigger>
					<Accordion.Content>
						<p class="text-small text-pretty text-muted-foreground">{item.answer}</p>
					</Accordion.Content>
				</Accordion.Item>
			{/each}
		</Accordion.Root>
	</div>
</Section>
