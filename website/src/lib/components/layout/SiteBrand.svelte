<script lang="ts">
	import { siteConfig } from '$lib/site/site';
</script>

<span class="inline-flex items-center gap-2.5 text-start text-title-1 font-medium">
	<span class="size-5.5 text-background [&>svg]:size-full shadow-sm rounded-sm bg-primary p-1" aria-hidden="true"
		>{@html siteConfig.logoRaw}</span
	>
	{siteConfig.name}
</span>
