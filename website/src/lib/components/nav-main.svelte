<script lang="ts">
	import { resolve } from '$app/paths';
	import * as Sidebar from '$lib/components/ui/sidebar';

	type NavigationItem = {
		title: string;
		url: string;
		isActive?: boolean;
	};

	type NavigationGroup = {
		id: string;
		title: string;
		items: readonly NavigationItem[];
	};

	let { groups, label }: { groups: readonly NavigationGroup[]; label: string } = $props();
</script>

<nav
	aria-label={label}
	data-docs-navigation
	class="flex scroll-py-4 scrollbar-none flex-col overflow-auto"
>
	{#each groups as group (group.id)}
		<Sidebar.Group class="shrink-0">
			<Sidebar.GroupLabel class="text-sm font-medium text-muted-foreground">
				{group.title}
			</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu class="gap-0.5">
					{#each group.items as item (item.url)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton tooltipContent={item.title} isActive={item.isActive ?? false}>
								{#snippet child({ props })}
									<a
										href={resolve(item.url.slice(1) as `${string}/${string}`)}
										aria-current={item.isActive ? 'page' : undefined}
										{...props}
									>
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	{/each}
</nav>
