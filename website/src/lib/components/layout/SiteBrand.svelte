<script lang="ts">
	import { siteConfig } from '$lib/site/site';
</script>

<span class="inline-flex items-center gap-2.5 text-start text-lg font-medium tracking-tight">
	<span class="size-6 text-foreground [&>svg]:size-full shadow-sm rounded-sm bg-card p-1.5" aria-hidden="true"
		>{@html siteConfig.logoRaw}</span
	>
	{siteConfig.name}
</span>
