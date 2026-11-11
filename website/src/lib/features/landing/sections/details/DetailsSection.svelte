<script lang="ts">
	import Section from '../../shared/Section.svelte';
	import SectionHeader from '../../shared/SectionHeader.svelte';
	import OutputDetails from './OutputDetails.svelte';
</script>

<Section aria-labelledby="details-title" divided>
	<SectionHeader
		id="details-title"
		description="A format is only the beginning. Choose what your output keeps, how it sounds, and which information comes along."
	>
		{#snippet title()}Every detail.<br />Still in your hands.{/snippet}
	</SectionHeader>
	<OutputDetails />
</Section>
