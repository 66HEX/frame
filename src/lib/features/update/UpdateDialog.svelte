<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { marked } from 'marked';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';
	import { updateStore } from '$lib/stores/update.svelte';

	let {
		onUpdate,
		onCancel
	}: {
		onUpdate: () => void;
		onCancel: () => void;
	} = $props();
</script>

{#if updateStore.showDialog}
	<div
		transition:fade={{ duration: 100 }}
		class="absolute inset-0 z-100 flex items-center justify-center bg-background/60 backdrop-blur-sm"
	>
		<div
			transition:scale={{ start: 1.05, duration: 100, opacity: 1 }}
			class="flex w-100 flex-col gap-4 rounded-lg border border-blue-600 bg-blue-900/20 p-3 shadow-2xl backdrop-blur-sm"
		>
			<div>
				<Label variant="section" class="text-foreground">{$_('update.available')}</Label>

				<p class="text-[10px] font-medium tracking-wide text-gray-alpha-600 uppercase">
					{$_('update.versionAvailable', { values: { version: updateStore.version } })}
				</p>
			</div>

			{#if updateStore.body}
				<div
					class="markdown-content max-h-35 overflow-y-auto rounded bg-gray-alpha-100 p-3 text-xs tracking-wide text-gray-alpha-600 uppercase"
				>
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html marked.parse(updateStore.body)}
				</div>
			{/if}

			{#if updateStore.error}
				<div class="text-xs text-red-600">
					{updateStore.error}
				</div>
			{/if}

			{#if updateStore.isInstalling}
				<div class="space-y-1">
					<div class="h-1 w-full overflow-hidden rounded-full bg-gray-alpha-200">
						<div
							class="h-full bg-blue-600 transition-all duration-300"
							style="width: {updateStore.progress}%"
						></div>
					</div>
					<p class="text-right text-[10px] text-gray-alpha-600">
						{Math.round(updateStore.progress)}%
					</p>
				</div>
			{:else}
				<div class="flex justify-end gap-2">
					<Button variant="ghost" onclick={onCancel}>{$_('update.later')}</Button>
					<Button onclick={onUpdate}>{$_('update.updateNow')}</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	:global(.markdown-content h1),
	:global(.markdown-content h2),
	:global(.markdown-content h3) {
		font-size: 11px;
		font-weight: 500;
		color: var(--foreground);
		margin-top: 1em;
		margin-bottom: 0.5em;
	}

	:global(.markdown-content h1:first-child),
	:global(.markdown-content h2:first-child),
	:global(.markdown-content h3:first-child) {
		margin-top: 0;
	}

	:global(.markdown-content ul) {
		list-style-type: disc;
		padding-left: 1.5em;
		margin-bottom: 0.5em;
	}

	:global(.markdown-content li) {
		margin-bottom: 0.25em;
		font-size: 10px;
	}

	:global(.markdown-content p) {
		margin-bottom: 0.5em;
	}

	:global(.markdown-content strong) {
		font-weight: 600;
		color: var(--foreground);
	}
</style>
