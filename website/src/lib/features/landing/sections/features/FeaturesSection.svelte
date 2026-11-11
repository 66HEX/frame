<script lang="ts">
	import Section from '../../shared/Section.svelte';
	import MediaFeatureCards from './MediaFeatureCards.svelte';
</script>

<Section id="features" aria-labelledby="features-title">
	<h2
		id="features-title"
		class="max-w-5xl text-3xl leading-tight font-medium tracking-tight text-pretty md:text-4xl"
	>
		A little more control. A lot less friction. <span class="text-muted-foreground"
			>Frame brings your media tools together in one calm, familiar workspace.</span
		>
	</h2>
	<div class="mt-14 md:mt-20"><MediaFeatureCards /></div>
</Section>
