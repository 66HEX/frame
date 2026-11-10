<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as Command from '$lib/components/ui/command';
	import { keyboardShortcuts, matchesKeyboardShortcut } from '$lib/site/keyboard-shortcuts';
	import { formatUiText, type SectionUiConfig } from '$lib/site/content-ui';
	import { ArrowTurnRightIcon, HashtagIcon, MagnifierIcon } from '$lib/icons';
	import {
		highlightDocsSearchMatch,
		searchDocsIndex,
		type DocsSearchIndex,
		type DocsSearchResult
	} from './docs-search';

	let { sectionId, config }: { sectionId: string; config: SectionUiConfig['search'] } = $props();
	let open = $state(false);
	let trigger = $state<HTMLElement | null>(null);
	let navigating = false;
	let query = $state('');
	let selectedValue = $state('');
	let searchIndex = $state.raw<DocsSearchIndex | null>(null);
	let loading = $state(false);
	let loadFailed = $state(false);
	const resultGroups = $derived(searchIndex ? searchDocsIndex(searchIndex, query, config) : []);
	const resultCount = $derived(
		resultGroups.reduce((count, group) => count + group.children.length + 1, 0)
	);
	const resultStatus = $derived.by(() => {
		if (loading) return config.loading;
		if (loadFailed) return config.error;
		if (!query.trim()) return '';
		if (resultCount === 0) return formatUiText(config.noResults, { query: query.trim() });
		return formatUiText(config.results, { count: resultCount });
	});

	function openSearch() {
		navigating = false;
		query = '';
		selectedValue = '';
		open = true;
		void loadSearchIndex();
	}

	async function loadSearchIndex() {
		if (searchIndex || loading) return;
		loading = true;
		loadFailed = false;
		try {
			searchIndex = (await import('./docs-search-index')).getSearchIndex(sectionId);
		} catch {
			loadFailed = true;
		} finally {
			loading = false;
		}
	}

	function closeSearch() {
		open = false;
	}

	function handleShortcut(event: KeyboardEvent) {
		if (
			!matchesKeyboardShortcut(event, keyboardShortcuts.search, {
				allowEditableTarget: true
			})
		) {
			return;
		}

		event.preventDefault();
		if (open) closeSearch();
		else openSearch();
	}

	function closeForNavigation() {
		navigating = true;
		closeSearch();
		query = '';
		selectedValue = '';
	}

	function restoreTriggerFocus(event: Event) {
		event.preventDefault();
		if (!navigating) trigger?.focus({ preventScroll: true });
	}

	function resultLabel(result: DocsSearchResult) {
		if (result.matchType === 'page') return result.title;
		if (result.matchType === 'heading') return result.heading ?? result.title;
		return result.snippet ?? result.heading ?? result.title;
	}

	onMount(() => {
		window.addEventListener('keydown', handleShortcut);
		return () => window.removeEventListener('keydown', handleShortcut);
	});

	afterNavigate(() => {
		closeForNavigation();
	});
</script>

<Button
	bind:ref={trigger}
	variant="outline"
	size="sm"
	class="aspect-square justify-center px-0 text-muted-foreground hover:bg-card sm:aspect-auto sm:w-44 sm:justify-start sm:px-2.5 lg:w-56 dark:hover:bg-input/30"
	aria-label={config.label}
	aria-keyshortcuts={keyboardShortcuts.search.ariaKeyShortcuts}
	aria-haspopup="dialog"
	aria-expanded={open}
	onclick={openSearch}
>
	<MagnifierIcon aria-hidden="true" class="size-4" />
	<span class="hidden min-w-0 flex-1 truncate text-start sm:block">{config.label}</span>
	<kbd
		class="pointer-events-none ms-auto hidden rounded-sm bg-muted px-1.5 font-mono text-[10px] leading-5 font-medium text-muted-foreground shadow-2xs lg:inline-flex"
		aria-hidden="true"
	>
		{keyboardShortcuts.search.display}
	</kbd>
</Button>

<Dialog.Root bind:open>
	<Dialog.Content
		onCloseAutoFocus={restoreTriggerFocus}
		showCloseButton={false}
		class="top-[10svh] max-h-[calc(100svh-2rem)] max-w-[calc(100%-2rem)] translate-y-0 overflow-hidden overscroll-contain p-0 sm:max-w-xl"
	>
		<Dialog.Header class="sr-only">
			<Dialog.Title>{config.label}</Dialog.Title>
			<Dialog.Description>{config.description}</Dialog.Description>
		</Dialog.Header>
		<Command.Root bind:value={selectedValue} shouldFilter={false} loop vimBindings={false}>
			<Command.Input
				bind:value={query}
				placeholder={config.placeholder}
				aria-label={config.label}
				class="text-base sm:text-sm"
			/>

			<p role="status" class="sr-only">{resultStatus}</p>

			<Command.List
				aria-busy={loading}
				style="mask-image: linear-gradient(to bottom, transparent, black 16px, black calc(100% - 16px), transparent); -webkit-mask-image: linear-gradient(to bottom, transparent, black 16px, black calc(100% - 16px), transparent);"
				class="max-h-[min(24rem,calc(100svh-9rem))] overscroll-contain py-1"
			>
				{#if loading}
					<Command.Empty forceMount class="px-6 text-muted-foreground">
						{config.loading}
					</Command.Empty>
				{:else if loadFailed}
					<Command.Empty forceMount class="space-y-3 px-6 text-muted-foreground">
						<p>{config.error}</p>
						<Button variant="outline" size="sm" onclick={loadSearchIndex}>{config.retry}</Button>
					</Command.Empty>
				{:else if !query.trim()}
					<Command.Empty forceMount class="px-6 text-muted-foreground">
						{config.emptyHint}
					</Command.Empty>
				{:else if resultGroups.length === 0}
					<Command.Empty forceMount class="px-6 text-muted-foreground">
						{formatUiText(config.noResults, { query: query.trim() })}
					</Command.Empty>
				{:else}
					{#each resultGroups as group (group.page.href)}
						<Command.Group value={group.page.href}>
							<Command.LinkItem
								href={group.page.href}
								value={group.page.id}
								onSelect={closeForNavigation}
								class="min-h-8 px-3 py-2"
								data-docs-search-result="page"
							>
								<div class="flex min-w-0 flex-1 items-baseline gap-2">
									<span class="min-w-0 truncate font-medium">
										{#each highlightDocsSearchMatch(group.page.title, query) as part, index (index)}
											<span class:text-primary={part.highlighted}>{part.text}</span>
										{/each}
									</span>
									<span class="ms-auto shrink-0 text-xs text-muted-foreground">
										{group.page.section}
									</span>
								</div>
							</Command.LinkItem>

							{#each group.children as result (result.id)}
								<Command.LinkItem
									href={result.href}
									value={result.id}
									onSelect={closeForNavigation}
									class="min-h-8 items-start py-2 ps-3 pe-3"
									data-docs-search-result={result.matchType}
								>
									<span class="mt-0.5 shrink-0 text-muted-foreground" aria-hidden="true">
										{#if result.matchType === 'heading'}
											<HashtagIcon size={16} />
										{:else}
											<ArrowTurnRightIcon size={16} />
										{/if}
									</span>
									<span class="min-w-0 flex-1">
										{#if result.matchType === 'heading'}
											<span class="block truncate font-medium">
												{#each highlightDocsSearchMatch(result.heading ?? result.title, query) as part, index (index)}
													<span class:text-primary={part.highlighted}>{part.text}</span>
												{/each}
											</span>
										{:else}
											{#if result.heading}
												<span class="sr-only">{result.heading}: </span>
											{/if}
											<span class="line-clamp-2 text-sm leading-5 text-muted-foreground">
												{#each highlightDocsSearchMatch(resultLabel(result), query) as part, index (index)}
													<span class:text-primary={part.highlighted}>{part.text}</span>
												{/each}
											</span>
										{/if}
									</span>
								</Command.LinkItem>
							{/each}
						</Command.Group>
					{/each}
				{/if}
			</Command.List>
		</Command.Root>
	</Dialog.Content>
</Dialog.Root>
