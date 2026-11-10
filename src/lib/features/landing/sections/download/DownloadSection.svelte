<script lang="ts">
	import { releaseUrl } from '../../data/content';
	import Section from '../../shared/Section.svelte';
	import SectionTitle from '../../shared/SectionTitle.svelte';
	import TextLink from '../../shared/TextLink.svelte';
	import DownloadCards from './DownloadCards.svelte';
</script>

<Section id="download" aria-labelledby="download-title" divided>
	<div class="grid items-start gap-12 lg:grid-cols-2 lg:gap-16">
		<div>
			<SectionTitle id="download-title">Make your next conversion a good one.</SectionTitle>
			<p class="mt-6 max-w-sm text-base leading-relaxed text-pretty text-muted-foreground md:text-xl md:leading-normal">
				Free and open source.<br />Pick your platform and bring your files.
			</p>
			<div class="mt-6 flex flex-wrap gap-x-6 gap-y-1">
				<TextLink href="/docs/installation">Installation instructions</TextLink>
				<TextLink href={releaseUrl}>All releases</TextLink>
			</div>
		</div>
		<DownloadCards />
	</div>
</Section>
