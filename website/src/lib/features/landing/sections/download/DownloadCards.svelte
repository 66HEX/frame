<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { downloads, releaseUrl } from '../../data/content';
	import TextLink from '../../shared/TextLink.svelte';
	const extensions: Record<string, string> = { macOS: '.dmg', Windows: '.exe', Linux: '.AppImage' };
</script>

<div class="download-platforms min-w-0 divide-y divide-border">
	{#each downloads as platform (platform.name)}
		<article
			class="flex min-w-0 flex-col gap-5 py-6 first:pt-0 last:pb-0 sm:flex-row sm:items-center sm:justify-between"
		>
			<div class="min-w-0">
				<h3 class="text-lg font-medium tracking-tight">{platform.name}</h3>
				<p class="mt-1 text-sm leading-6 text-muted-foreground">{platform.description}</p>
				{#if platform.name === 'Linux'}
					<TextLink href="/docs/installation#nixpkgs">Install with Nixpkgs</TextLink>
				{/if}
			</div>
			<div class="flex shrink-0 flex-wrap gap-2">
				{#each platform.options as option (option.file)}
					<Button
						href={`${releaseUrl}/download/${option.file}`}
						variant="outline"
						size="default"
						aria-label={`Download Frame for ${platform.name}: ${option.label}, ${extensions[platform.name]}`}
					>
						{platform.name === 'Windows' ? 'Download' : option.label}
					</Button>
				{/each}
			</div>
		</article>
	{/each}
</div>
