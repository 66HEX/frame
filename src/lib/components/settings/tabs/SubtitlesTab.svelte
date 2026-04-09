<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, SourceMetadata } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import ColorPicker from '$lib/components/ui/ColorPicker.svelte';
	import { _ } from '$lib/i18n';
	import { openNativeFileDialog } from '$lib/services/dialog';
	import { IconClose } from '$lib/icons';
	import { fontsStore } from '$lib/stores/fonts.svelte';

	const POSITIONS = ['bottom', 'middle', 'top'] as const;
	const FONT_SIZES = ['8', '10', '12', '14', '16', '18', '20', '22', '24', '28', '32', '36', '42', '48'] as const;

	let {
		config,
		disabled = false,
		copyMode = false,
		onUpdate,
		metadata
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		copyMode?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		metadata?: SourceMetadata;
	} = $props();

	const burnInDisabled = $derived(disabled || copyMode);

	$effect(() => {
		if (!copyMode && !disabled) {
			fontsStore.load();
		}
	});

	function toggleTrack(index: number) {
		if (disabled) return;
		const current = config.selectedSubtitleTracks || [];
		if (current.includes(index)) {
			onUpdate({
				selectedSubtitleTracks: current.filter((i) => i !== index)
			});
		} else {
			onUpdate({ selectedSubtitleTracks: [...current, index] });
		}
	}

	async function selectExternalSubtitle() {
		if (burnInDisabled) return;
		const selected = await openNativeFileDialog({
			multiple: false,
			filters: [
				{
					name: 'Subtitles',
					extensions: ['srt', 'ass', 'vtt']
				}
			]
		});

		if (selected && typeof selected === 'string') {
			onUpdate({ subtitleBurnPath: selected });
		}
	}

	function clearExternalSubtitle() {
		if (burnInDisabled) return;
		onUpdate({ subtitleBurnPath: undefined });
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('subtitles.burnIn')}</Label>
		<div class="space-y-3">
			<div class="relative flex items-center">
				<Button
					variant="secondary"
					disabled={burnInDisabled}
					onclick={selectExternalSubtitle}
					class={cn('w-full transition-colors', config.subtitleBurnPath ? 'pr-8' : '')}
				>
					<span class={cn('truncate text-foreground')}>
						{config.subtitleBurnPath
							? config.subtitleBurnPath.split(/[\\/]/).pop()
							: $_('subtitles.selectFile')}
					</span>
				</Button>

				{#if config.subtitleBurnPath}
					<div class="absolute right-3 flex items-center">
						<Button
							variant="destructive"
							size="none"
							class="size-5"
							onclick={(e) => {
								e.stopPropagation();
								clearExternalSubtitle();
							}}
							disabled={burnInDisabled}
							title={$_('subtitles.clearFile')}
						>
							<IconClose size={12} />
						</Button>
					</div>
				{/if}
			</div>
			<p class="text-[10px] text-gray-alpha-600">
				{copyMode ? $_('subtitles.copyModeHint') : $_('subtitles.burnInHint')}
			</p>
		</div>
	</div>

	{#if !copyMode}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('subtitles.style')}</Label>
			<div class="space-y-3">
				<div class="grid grid-cols-2 gap-2">
					<div class="space-y-2">
						<Label for="subtitle-font">{$_('subtitles.fontName')}</Label>
						<Select
							id="subtitle-font"
							value={config.subtitleFontName ?? ''}
							options={fontsStore.fonts}
							placeholder={$_('subtitles.fontNamePlaceholder')}
							disabled={burnInDisabled}
							onchange={(v) => onUpdate({ subtitleFontName: v || undefined })}
						/>
					</div>
					<div class="space-y-2">
						<Label for="subtitle-font-size">{$_('common.size')}</Label>
						<Select
							id="subtitle-font-size"
							value={config.subtitleFontSize ?? ''}
							options={[...FONT_SIZES]}
							placeholder="Default"
							disabled={burnInDisabled}
							onchange={(v) => onUpdate({ subtitleFontSize: v || undefined })}
						/>
					</div>
				</div>

				<div class="grid grid-cols-2 gap-2">
					<div class="space-y-2">
						<Label for="subtitle-font-color">{$_('subtitles.fontColor')}</Label>
						<ColorPicker
							id="subtitle-font-color"
							value={config.subtitleFontColor ?? '#ffffff'}
							onchange={(v) => onUpdate({ subtitleFontColor: v })}
							disabled={burnInDisabled}
						/>
					</div>
					<div class="space-y-2">
						<Label for="subtitle-outline-color">{$_('subtitles.outlineColor')}</Label>
						<ColorPicker
							id="subtitle-outline-color"
							value={config.subtitleOutlineColor ?? '#000000'}
							onchange={(v) => onUpdate({ subtitleOutlineColor: v })}
							disabled={burnInDisabled}
						/>
					</div>
				</div>

				<div class="space-y-2">
					<Label>{$_('subtitles.position')}</Label>
					<div class="grid grid-cols-3 gap-2">
						{#each POSITIONS as pos (pos)}
							<Button
								variant={(config.subtitlePosition ?? 'bottom') === pos ? 'default' : 'secondary'}
								onclick={() => onUpdate({ subtitlePosition: pos })}
								disabled={burnInDisabled}
								class="w-full"
							>
								{$_(`subtitles.position${pos.charAt(0).toUpperCase() + pos.slice(1)}`)}
							</Button>
						{/each}
					</div>
				</div>

				<p class="text-[10px] text-gray-alpha-600">{$_('subtitles.styleHint')}</p>
			</div>
		</div>
	{/if}

	{#if metadata?.subtitleTracks && metadata.subtitleTracks.length > 0}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('subtitles.sourceTracks')}</Label>
			<div class="grid grid-cols-1 gap-2">
				{#each metadata.subtitleTracks as track (track.index)}
					{@const isSelected = (config.selectedSubtitleTracks || []).includes(track.index)}
					<Button
						variant={isSelected ? 'default' : 'secondary'}
						onclick={() => toggleTrack(track.index)}
						{disabled}
						class="flex h-auto w-full items-center justify-between px-3 py-2 text-left"
					>
						<div class="space-y-0.5">
							<div class="flex items-center gap-2">
								<span class="text-[10px] opacity-70">
									#{track.index}
								</span>
								<span class="text-[10px] font-semibold">
									{track.codec}
								</span>
								<div class="text-[10px]">
									{#if track.language}
										<span class="mx-0.5">•</span>
										{track.language}{/if}
									{#if track.label}
										<span class="mx-0.5">•</span>
										{track.label}{/if}
								</div>
							</div>
						</div>

						<div
							class={cn(
								'input-highlight flex h-3 w-3 items-center justify-center rounded-full transition-all'
							)}
						>
							<div
								class="h-1.5 w-1.5 rounded-full bg-blue-700 transition-all"
								style="opacity: {isSelected ? 1 : 0}; transform: scale({isSelected ? 1 : 0.5});"
							></div>
						</div>
					</Button>
				{/each}
			</div>
		</div>
	{:else}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('subtitles.sourceTracks')}</Label>
			<p class="text-[10px] text-gray-alpha-600">
				{$_('subtitles.none')}
			</p>
		</div>
	{/if}
</div>
