<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, SourceMetadata } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';
	import { openNativeFileDialog } from '$lib/services/dialog';
	import { IconClose } from '$lib/icons';

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
		if (disabled) return;
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
		if (disabled) return;
		onUpdate({ subtitleBurnPath: undefined });
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('subtitles.burnIn')}</Label>
		<div class="space-y-3">
			<div class="relative flex items-center">
				<Button
					variant="outline"
					{disabled}
					onclick={selectExternalSubtitle}
					class={cn('w-full transition-colors', config.subtitleBurnPath ? 'pr-8' : '')}
				>
					<span
						class={cn(
							'truncate',
							config.subtitleBurnPath ? 'text-foreground' : 'text-gray-alpha-600'
						)}
					>
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
							class="h-4 w-4 text-gray-alpha-600 hover:bg-transparent hover:text-red-600 disabled:pointer-events-none disabled:opacity-50"
							onclick={(e) => {
								e.stopPropagation();
								clearExternalSubtitle();
							}}
							{disabled}
							title={$_('subtitles.clearFile')}
						>
							<IconClose size={14} />
						</Button>
					</div>
				{/if}
			</div>
			<p class="text-[9px] text-gray-alpha-600">
				{$_('subtitles.burnInHint')}
			</p>
		</div>
	</div>

	{#if metadata?.subtitleTracks && metadata.subtitleTracks.length > 0}
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('subtitles.sourceTracks')}</Label>
			<div class="grid grid-cols-1 gap-2">
				{#each metadata.subtitleTracks as track (track.index)}
					{@const isSelected = (config.selectedSubtitleTracks || []).includes(track.index)}
					<Button
						variant={isSelected ? 'selected' : 'outline'}
						onclick={() => toggleTrack(track.index)}
						{disabled}
						class="flex h-auto w-full items-center justify-between px-3 py-2 text-left"
					>
						<div class="space-y-0.5">
							<div class="flex items-center gap-2">
								<span class="text-[10px] opacity-70">
									#{track.index}
								</span>
								<span class="text-[10px] font-medium tracking-wide">
									{track.codec}
								</span>
								<div class="text-[9px] tracking-wide">
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
								'flex h-3 w-3 items-center justify-center rounded-full border transition-all',
								isSelected ? 'border-blue-600' : 'border-gray-alpha-200'
							)}
						>
							<div
								class="h-1.5 w-1.5 rounded-full bg-blue-600 transition-all"
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
			<p class="text-[9px] text-gray-alpha-600">
				{$_('subtitles.none')}
			</p>
		</div>
	{/if}
</div>
