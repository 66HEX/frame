<script lang="ts">
	import Section from '../../shared/Section.svelte';
	import SectionHeader from '../../shared/SectionHeader.svelte';
	import FeatureLinks from '../../shared/FeatureLinks.svelte';
	import PreviewShowcase from './PreviewShowcase.svelte';
</script>

<Section id="preview" aria-labelledby="preview-title" divided>
	<SectionHeader
		id="preview-title"
		description="Trim the moment. Find the right crop. Adjust your output with the preview and controls in view, so every decision stays connected to your media."
		href="/docs/preview"
		linkLabel="Explore the preview"
	>
		{#snippet title()}See the change.<br />Before you convert.{/snippet}
	</SectionHeader>
	<PreviewShowcase />
	<FeatureLinks
		description="Make room for the details."
		links={[
			{ label: 'Crop & trim', href: '/docs/preview' },
			{ label: 'Video filters', href: '/docs/video' },
			{ label: 'Image output', href: '/docs/images' },
			{ label: 'Encoding controls', href: '/docs/video' }
		]}
	/>
</Section>
