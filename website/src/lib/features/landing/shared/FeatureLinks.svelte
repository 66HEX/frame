<script lang="ts">
	import { ArrowRightIcon } from '$lib/icons';

	let {
		description,
		links
	}: {
		description: string;
		links: { label: string; href: string }[];
	} = $props();
</script>

<div class="mt-10 grid gap-4 text-sm md:mt-14 md:grid-cols-2 md:gap-16">
	<p class="text-muted-foreground">{description}</p>
	<div class="grid grid-cols-2 gap-x-6 gap-y-1">
		{#each links as link (link.label)}
			<a
				href={link.href}
				class="flex min-h-10 items-center justify-between gap-2 text-muted-foreground transition-colors hover:text-foreground"
			>
				{link.label}<ArrowRightIcon class="size-3.5" />
			</a>
		{/each}
	</div>
</div>
