<script lang="ts">
	import Section from '../../shared/Section.svelte';
	import SectionHeader from '../../shared/SectionHeader.svelte';
	import FeatureLinks from '../../shared/FeatureLinks.svelte';
	import WorkflowShowcase from './WorkflowShowcase.svelte';
</script>

<Section id="workflow" aria-labelledby="workflow-title" divided>
	<SectionHeader
		id="workflow-title"
		description="Bring your files into one queue. Start with a preset, tune the output, and apply your settings across a batch. Save what works for the next time."
		href="/docs/batch-conversion"
		linkLabel="Explore batch conversion"
	>
		{#snippet title()}One file.<br />Or the whole folder.{/snippet}
	</SectionHeader>
	<WorkflowShowcase />
	<FeatureLinks
		description="Less setup. More doing."
		links={[
			{ label: 'Batch processing', href: '/docs/batch-conversion' },
			{ label: 'Built-in presets', href: '/docs/presets' },
			{ label: 'Custom presets', href: '/docs/presets' },
			{ label: 'Your first conversion', href: '/docs/getting-started' }
		]}
	/>
</Section>
