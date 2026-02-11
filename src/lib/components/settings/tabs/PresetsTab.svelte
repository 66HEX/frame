<script lang="ts">
	import { onDestroy } from 'svelte';
	import { IconTrash, IconListChecks } from '$lib/icons';
	import { askNativeDialog } from '$lib/services/dialog';
	import { cn } from '$lib/utils/cn';
	import {
		AUDIO_ONLY_CONTAINERS,
		type ConversionConfig,
		type PresetDefinition,
		type SourceMetadata
	} from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import ListItem from '$lib/components/ui/ListItem.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';

	let {
		config,
		presets = [],
		metadata,
		disabled = false,
		onApplyPreset,
		onApplyPresetToAll,
		onSavePreset,
		onDeletePreset
	}: {
		config: ConversionConfig;
		presets?: PresetDefinition[];
		metadata?: SourceMetadata;
		disabled?: boolean;
		onApplyPreset?: (preset: PresetDefinition) => void;
		onApplyPresetToAll?: (preset: PresetDefinition) => void;
		onSavePreset?: (name: string) => Promise<boolean | void> | boolean | void;
		onDeletePreset?: (id: string) => Promise<boolean | void> | boolean | void;
	} = $props();

	let newPresetName = $state('');
	type NoticeTone = 'success' | 'error';
	let notice = $state<{ text: string; tone: NoticeTone } | null>(null);
	let noticeTimeout: ReturnType<typeof setTimeout> | null = null;

	const isSourceAudioOnly = $derived(!!metadata && !metadata.videoCodec);

	onDestroy(() => {
		if (noticeTimeout) clearTimeout(noticeTimeout);
	});

	function configsMatch(a: ConversionConfig, b: ConversionConfig) {
		const baseMatch =
			a.container === b.container &&
			a.videoCodec === b.videoCodec &&
			a.audioCodec === b.audioCodec &&
			a.resolution === b.resolution &&
			a.preset === b.preset &&
			a.videoBitrateMode === b.videoBitrateMode;

		if (!baseMatch) return false;

		if (a.videoBitrateMode === 'crf') {
			if (a.crf !== b.crf || a.quality !== b.quality) return false;
		} else {
			if (a.videoBitrate !== b.videoBitrate) return false;
		}

		if (a.resolution === 'custom') {
			if (a.customWidth !== b.customWidth || a.customHeight !== b.customHeight) return false;
		}

		return true;
	}

	function showNotice(text: string, tone: NoticeTone = 'success') {
		notice = { text, tone };
		if (noticeTimeout) clearTimeout(noticeTimeout);
		noticeTimeout = setTimeout(() => (notice = null), 2400);
	}

	async function savePreset() {
		if (!onSavePreset || disabled) return;
		if (!newPresetName.trim()) {
			showNotice($_('presets.nameRequired'), 'error');
			return;
		}

		const result = await onSavePreset(newPresetName.trim());
		if (result === false) {
			showNotice($_('presets.notSaved'), 'error');
			return;
		}

		newPresetName = '';
		showNotice($_('presets.saved'));
	}

	function applyPreset(preset: PresetDefinition) {
		if (disabled) return;
		onApplyPreset?.(preset);
		showNotice($_('presets.appliedName', { values: { name: preset.name } }));
	}

	async function handleApplyToAll(preset: PresetDefinition) {
		if (disabled || !onApplyPresetToAll) return;

		const confirmed = await askNativeDialog({
			message: $_('presets.confirmApplyAllBody', { values: { name: preset.name } }),
			title: $_('presets.confirmApplyAllTitle'),
			kind: 'warning',
			okLabel: $_('common.apply'),
			cancelLabel: $_('common.cancel')
		});

		if (confirmed) {
			onApplyPresetToAll(preset);
			showNotice($_('presets.appliedAll'));
		}
	}

	async function removePreset(preset: PresetDefinition) {
		if (!onDeletePreset || preset.builtIn) return;
		const result = await onDeletePreset(preset.id);
		if (result === false) {
			showNotice($_('presets.unableToDelete'), 'error');
			return;
		}

		showNotice($_('presets.removed'));
	}
</script>

<div class="space-y-3">
	<div class="relative w-full">
		<Label variant="section">{$_('presets.library')}</Label>
		{#if notice}
			<span
				class={cn(
					'absolute top-0 right-0 text-[9px] tracking-wide',
					notice.tone === 'error' ? 'text-red-700' : 'text-blue-600'
				)}
			>
				{notice.text}
			</span>
		{/if}
	</div>

	<div class="flex gap-2">
		<div class="flex-1">
			<Input
				type="text"
				value={newPresetName}
				oninput={(e) => (newPresetName = e.currentTarget.value)}
				placeholder={$_('presets.label')}
				{disabled}
			/>
		</div>
		<Button onclick={savePreset} disabled={disabled || !newPresetName.trim()} variant="outline">
			{$_('common.save')}
		</Button>
	</div>

	<div class="grid grid-cols-1">
		{#each presets as preset (preset.id)}
			{@const isCompatible =
				!isSourceAudioOnly || AUDIO_ONLY_CONTAINERS.includes(preset.config.container)}
			<ListItem
				selected={configsMatch(config, preset.config)}
				onclick={() => isCompatible && applyPreset(preset)}
				disabled={disabled || !isCompatible}
				onkeydown={(event) => {
					if (event.key === 'Enter' || event.key === ' ') {
						event.preventDefault();
						if (isCompatible) applyPreset(preset);
					}
				}}
				class={cn('pr-1', !isCompatible && 'pointer-events-none opacity-50')}
			>
				<span class="truncate">{preset.name}</span>
				<div class="flex items-center gap-2">
					<span class="pr-2 text-[9px] font-medium opacity-50">
						{#if !isCompatible}
							{$_('audio.incompatibleContainer')}
						{:else if configsMatch(config, preset.config)}
							{$_('presets.applied')}
						{/if}
					</span>

					{#if isCompatible}
						<Button
							variant="ghost"
							size="none"
							class="size-5 shrink-0 opacity-50 hover:opacity-100"
							title={$_('presets.applyToAll')}
							onclick={(event) => {
								event.stopPropagation();
								handleApplyToAll(preset);
							}}
							{disabled}
						>
							<IconListChecks size={14} />
						</Button>
					{/if}

					{#if !preset.builtIn}
						<Button
							variant="destructive"
							size="none"
							class="size-5 shrink-0 opacity-50 hover:opacity-100"
							title={$_('presets.deletePreset')}
							onclick={(event) => {
								event.stopPropagation();
								removePreset(preset);
							}}
							{disabled}
						>
							<IconTrash size={12} />
						</Button>
					{/if}
				</div>
			</ListItem>
		{/each}
	</div>
</div>
