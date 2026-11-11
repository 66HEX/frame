<script lang="ts">
	import LandingLayout from './layout/LandingLayout.svelte';
	import HeroSection from './sections/hero/HeroSection.svelte';
	import PreviewSection from './sections/preview/PreviewSection.svelte';
	import WorkflowSection from './sections/workflow/WorkflowSection.svelte';
	import DetailsSection from './sections/details/DetailsSection.svelte';
	import LocalSection from './sections/local/LocalSection.svelte';
	import DownloadSection from './sections/download/DownloadSection.svelte';
	import FaqSection from './sections/faq/FaqSection.svelte';
</script>

<LandingLayout>
	<HeroSection />
	<PreviewSection />
	<WorkflowSection />
	<DetailsSection />
	<LocalSection />
	<DownloadSection />
	<FaqSection />
</LandingLayout>
