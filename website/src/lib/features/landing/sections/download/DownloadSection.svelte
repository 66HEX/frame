<script lang="ts">
	import { ArrowRightIcon } from '$lib/icons';
	import { releaseUrl } from '../../data/content';
	import Section from '../../shared/Section.svelte';
	import SectionTitle from '../../shared/SectionTitle.svelte';
	import SectionDescription from '../../shared/SectionDescription.svelte';
	import TextLink from '../../shared/TextLink.svelte';
	import DownloadCards from './DownloadCards.svelte';
</script>

<Section id="download" aria-labelledby="download-title" divided>
	<div class="grid items-start gap-8 pb-12 md:grid-cols-2 md:gap-16 md:pb-16">
		<div>
			<SectionTitle id="download-title">Make your next conversion a good one.</SectionTitle>
		</div>
		<div>
			<SectionDescription>
				Free and open source. Pick your platform and bring your files.
			</SectionDescription>
			<div class="mt-4 flex flex-wrap gap-x-6 gap-y-1">
				<TextLink href="/docs/installation">
					Installation instructions<ArrowRightIcon class="size-4" aria-hidden="true" />
				</TextLink>
				<TextLink href={releaseUrl}>
					All releases<ArrowRightIcon class="size-4" aria-hidden="true" />
				</TextLink>
			</div>
		</div>
	</div>
	<DownloadCards />
</Section>
