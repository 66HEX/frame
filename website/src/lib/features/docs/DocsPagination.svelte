<script lang="ts">
	import type { SectionUiConfig } from '$lib/site/content-ui';
	import { resolve } from '$app/paths';
	import * as Card from '$lib/components/ui/card';
	import type { DocsPaginationItem } from '$lib/content/docs';
	import { ChevronRightIcon } from '$lib/icons';
	import { cn } from '$lib/utils';

	let {
		previous,
		next,
		config
	}: {
		previous: DocsPaginationItem | null;
		next: DocsPaginationItem | null;
		config: SectionUiConfig['pagination'];
	} = $props();

	const linkClass =
		'group flex flex-col justify-between gap-3 rounded-lg bg-card p-4 shadow-sm transition-[color,background-color,border-color,box-shadow,scale,background-size] outline-none focus-visible:ring-3 focus-visible:ring-ring/50 motion-reduce:transition-none hover:bg-[color-mix(in_lab,var(--color-card),var(--color-muted)_50%)]';
</script>

{#if previous || next}
	<nav class="mt-16 border-t border-border pt-8" aria-label={config.label}>
		<div class="grid gap-3 sm:grid-cols-2">
			{#if previous}
				<Card.Root size="sm" class="gap-0 bg-muted p-1">
					<a href={resolve(previous.href.slice(1) as `${string}/${string}`)} class={linkClass}>
						<span class="flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
							<ChevronRightIcon
								class="rotate-180 transition-transform duration-150 group-hover:-translate-x-0.5 motion-reduce:transition-none rtl:rotate-0"
								size={14}
								aria-hidden="true"
							/>
							{config.previous}
						</span>

						<span
							class="font-heading text-base leading-snug font-medium text-balance text-foreground"
						>
							{previous.title}
						</span>
					</a>
				</Card.Root>
			{/if}

			{#if next}
				<Card.Root size="sm" class={cn('gap-0 bg-muted p-1', !previous && 'sm:col-start-2')}>
					<a
						href={resolve(next.href.slice(1) as `${string}/${string}`)}
						class={cn(linkClass, 'items-end text-end')}
					>
						<span class="flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
							{config.next}
							<ChevronRightIcon
								class="transition-transform duration-150 group-hover:translate-x-0.5 motion-reduce:transition-none rtl:rotate-180"
								size={14}
								aria-hidden="true"
							/>
						</span>

						<span
							class="font-heading text-base leading-snug font-medium text-balance text-foreground"
						>
							{next.title}
						</span>
					</a>
				</Card.Root>
			{/if}
		</div>
	</nav>
{/if}
