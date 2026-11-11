<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { contentSections } from '$lib/content/sections';
	import type { ContentSectionConfig } from '$lib/content/types';
	import { contentUiDefaults } from '$lib/site/content-ui';
	import { CheckIcon, ChevronExpandYIcon } from '$lib/icons';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Button } from '$lib/components/ui/button';

	let { section }: { section: ContentSectionConfig } = $props();
	const label = contentUiDefaults.shell.viewSwitcherLabel;
	let hydrated = $state(false);
	onMount(() => {
		hydrated = true;
	});
</script>

{#if contentSections.length > 1}
	<div class="mx-2 mb-2" data-view-switcher>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						disabled={!hydrated}
						class="w-full"
						aria-label={`${label}: ${section.label}`}
					>
						<section.icon size={18} aria-hidden="true" />
						<span class="min-w-0 flex-1 truncate text-start">{section.label}</span>
						<ChevronExpandYIcon size={18} class="text-muted-foreground" aria-hidden="true" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start" sideOffset={6} class="min-w-56">
				<DropdownMenu.Group>
					<DropdownMenu.GroupHeading class="sr-only">{label}</DropdownMenu.GroupHeading>
					{#each contentSections as view (view.id)}
						<DropdownMenu.Item>
							{#snippet child({ props })}
								<a
									{...props}
									href={resolve('/[section]/[...slug]', { section: view.id, slug: '' })}
									aria-current={view.id === section.id ? 'page' : undefined}
								>
									<view.icon size={18} aria-hidden="true" />
									<span class="min-w-0 flex-1"
										><span class="block font-medium">{view.label}</span></span
									>
									{#if view.id === section.id}<CheckIcon size={16} aria-hidden="true" />{/if}
								</a>
							{/snippet}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Group>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>
{/if}
