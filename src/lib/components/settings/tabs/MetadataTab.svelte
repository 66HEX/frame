<script lang="ts">
	import type { ConversionConfig, MetadataMode, SourceMetadata } from '$lib/types';
	import Label from '$lib/components/ui/Label.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { _ } from '$lib/i18n';

	let {
		config,
		disabled = false,
		onUpdate,
		metadata
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		metadata?: SourceMetadata;
	} = $props();

	function updateMetadata<K extends keyof ConversionConfig['metadata']>(
		key: K,
		value: ConversionConfig['metadata'][K]
	) {
		const nextMetadata = { ...config.metadata, [key]: value };
		onUpdate({ metadata: nextMetadata });
	}

	function getPlaceholder(key: string): string {
		if (config.metadata.mode !== 'preserve') return '';

		const existingValue = metadata?.tags?.[key as keyof typeof metadata.tags];

		if (existingValue) {
			return existingValue;
		}

		return $_('metadata.placeholderPreserve');
	}

	const MODES: { value: MetadataMode; label: string }[] = [
		{ value: 'preserve', label: 'metadata.modes.preserve' },
		{ value: 'clean', label: 'metadata.modes.clean' },
		{ value: 'replace', label: 'metadata.modes.replace' }
	];

	const FIELDS = [
		{ key: 'title', label: 'metadata.fields.title' },
		{ key: 'artist', label: 'metadata.fields.artist' },
		{ key: 'album', label: 'metadata.fields.album' },
		{ key: 'genre', label: 'metadata.fields.genre' },
		{ key: 'date', label: 'metadata.fields.date' },
		{ key: 'comment', label: 'metadata.fields.comment' }
	] as const;
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('metadata.mode')}</Label>
		<div class="grid grid-cols-3 gap-2">
			{#each MODES as mode (mode.value)}
				<Button
					variant={config.metadata.mode === mode.value ? 'selected' : 'outline'}
					{disabled}
					class="w-full"
					onclick={() => updateMetadata('mode', mode.value)}
				>
					{$_(mode.label)}
				</Button>
			{/each}
		</div>
		<p class="text-[9px] text-gray-alpha-600 uppercase">
			{$_(`metadata.modes.${config.metadata.mode}Desc`)}
		</p>
	</div>

	{#if config.metadata.mode !== 'clean'}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('metadata.fieldsSection') || 'Fields'}</Label>
			<div class="space-y-3">
				{#each FIELDS as field (field.key)}
					<div class="space-y-2">
						<Label for="metadata-{field.key}">{$_(field.label)}</Label>
						<Input
							class="placeholder:normal-case"
							id="metadata-{field.key}"
							value={config.metadata[field.key] || ''}
							{disabled}
							placeholder={getPlaceholder(field.key)}
							oninput={(e) => updateMetadata(field.key, e.currentTarget.value)}
						/>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
