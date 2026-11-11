<script lang="ts">
	import { ArrowRightIcon, GitHubIcon, CpuIcon, CheckIcon, CodeIcon } from '$lib/icons';
	import { githubUrl } from '../../data/content';
	import Section from '../../shared/Section.svelte';
	import SectionTitle from '../../shared/SectionTitle.svelte';
	import SectionDescription from '../../shared/SectionDescription.svelte';
	import TextLink from '../../shared/TextLink.svelte';
	import LocalProcessingIllustration from './LocalProcessingIllustration.svelte';

	const benefits = [
		{
			icon: CpuIcon,
			title: 'Native from the start.',
			description: 'Built with Rust and GPUI. A desktop workspace that feels at home.'
		},
		{
			icon: CheckIcon,
			title: 'Your machine does the work.',
			description: 'Local conversion with FFmpeg. Hardware encoding on supported systems.'
		},
		{
			icon: CodeIcon,
			title: 'Open by default.',
			description: 'Free to use. Source available under GPLv3, for anyone to explore.'
		}
	];
</script>

<Section aria-labelledby="local-title" divided>
	<div class="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
		<div>
			<SectionTitle id="local-title">Local files.<br />Local processing.</SectionTitle>
			<SectionDescription class="mt-6"
				>Frame runs FFmpeg on your machine. Your footage, recordings, and images stay out of a cloud
				conversion queue.</SectionDescription
			>
			<TextLink href={githubUrl} class="mt-6"
				><GitHubIcon class="size-4" />See how Frame is built<ArrowRightIcon
					class="size-4"
				/></TextLink
			>
		</div>
		<LocalProcessingIllustration />
	</div>
	<div class="mt-16 grid gap-8 md:grid-cols-3 md:gap-12">
		{#each benefits as benefit (benefit.title)}
			<div>
				<benefit.icon class="mb-4 size-5 text-muted-foreground" />
				<h3 class="text-base font-medium">{benefit.title}</h3>
				<p class="mt-2 text-sm leading-6 text-muted-foreground">{benefit.description}</p>
			</div>
		{/each}
	</div>
</Section>
