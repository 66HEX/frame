<script lang="ts">
	import { ArrowRightIcon } from '$lib/icons';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { downloads, releaseUrl } from '../../data/content';
	import TextLink from '../../shared/TextLink.svelte';

	const extensions: Record<string, string> = {
		macOS: '.dmg',
		Windows: '.exe',
		Linux: '.AppImage'
	};
</script>

<div class="download-platforms grid min-w-0 gap-6 md:grid-cols-3">
	{#each downloads as platform (platform.name)}
		<Card.Root size="sm" class="min-w-0 gap-0 bg-muted px-1 pt-0 pb-1">
			<div class="flex h-11 items-center justify-between px-3">
				<h3 class="text-sm font-medium text-foreground">{platform.name}</h3>
				<span class="text-mini text-muted-foreground">{extensions[platform.name]}</span>
			</div>
			<div
				class="flex flex-1 flex-col justify-between gap-6 rounded-lg border border-border bg-card p-4"
			>
				<div>
					<p class="text-small text-pretty text-muted-foreground">
						{platform.description}
					</p>
					{#if platform.name === 'Linux'}
						<TextLink href="/docs/installation#nixpkgs" class="mt-2">
							Install with Nixpkgs<ArrowRightIcon class="size-4" aria-hidden="true" />
						</TextLink>
					{/if}
				</div>
				<div class="flex flex-wrap gap-2">
					{#each platform.options as option (option.file)}
						<Button
							href={`${releaseUrl}/download/${option.file}`}
							variant="secondary"
							size="default"
							aria-label={`Download Frame for ${platform.name}: ${option.label}, ${extensions[platform.name]}`}
						>
							{platform.name === 'Windows' ? 'Download' : option.label}
						</Button>
					{/each}
				</div>
			</div>
		</Card.Root>
	{/each}
</div>
