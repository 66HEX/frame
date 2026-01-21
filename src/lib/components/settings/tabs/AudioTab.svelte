<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, SourceMetadata } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import ListItem from '$lib/components/ui/ListItem.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';

	const AUDIO_CODECS = [
		{ id: 'aac', label: 'AAC / Stereo' },
		{ id: 'ac3', label: 'Dolby Digital' },
		{ id: 'libopus', label: 'Opus' },
		{ id: 'mp3', label: 'MP3' }
	] as const;

	const CHANNELS = [
		{ id: 'original', label: 'Original' },
		{ id: 'stereo', label: 'Stereo (2.0)' },
		{ id: 'mono', label: 'Mono (1.0)' }
	] as const;

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
		const current = config.selectedAudioTracks || [];
		if (current.includes(index)) {
			onUpdate({
				selectedAudioTracks: current.filter((i) => i !== index)
			});
		} else {
			onUpdate({ selectedAudioTracks: [...current, index] });
		}
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">Channels & Bitrate</Label>
		<div class="space-y-3">
			<div class="grid grid-cols-3 gap-2">
				{#each CHANNELS as ch (ch.id)}
					<Button
						variant={config.audioChannels === ch.id ? 'selected' : 'outline'}
						onclick={() => onUpdate({ audioChannels: ch.id })}
						{disabled}
						class="w-full"
					>
						{ch.label}
					</Button>
				{/each}
			</div>

			<div class="space-y-2 pt-1">
				<Label for="audio-bitrate">Bitrate (kbps)</Label>
				<Input
					id="audio-bitrate"
					type="number"
					value={config.audioBitrate}
					oninput={(e) => onUpdate({ audioBitrate: e.currentTarget.value })}
					{disabled}
				/>
			</div>
		</div>
	</div>
	<div class="space-y-3 pt-2">
		<Label variant="section">Audio Codec</Label>
		<div class="grid grid-cols-1">
			{#each AUDIO_CODECS as codec (codec.id)}
				{@const isMp3Container = config.container === 'mp3'}
				{@const isAllowed = !isMp3Container || codec.id === 'mp3'}
				<ListItem
					selected={config.audioCodec === codec.id}
					onclick={() => onUpdate({ audioCodec: codec.id })}
					disabled={disabled || !isAllowed}
					class={cn(!isAllowed && 'cursor-not-allowed opacity-50')}
				>
					<span>{codec.id}</span>
					<span class="text-[9px] opacity-50">
						{isMp3Container && codec.id !== 'mp3' ? 'Incompatible with MP3' : codec.label}
					</span>
				</ListItem>
			{/each}
		</div>
	</div>

	{#if metadata?.audioTracks && metadata.audioTracks.length > 0}
		<div class="space-y-3 pt-2">
			<Label variant="section">Source Tracks</Label>
			<div class="grid grid-cols-1 gap-2">
				{#each metadata.audioTracks as track (track.index)}
					{@const isSelected = (config.selectedAudioTracks || []).includes(track.index)}
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
								<span class="text-[10px] font-bold tracking-tight uppercase">
									{track.codec}
								</span>
								<div class="text-[9px] tracking-wide uppercase">
									<span class="mx-0.5">•</span>

									{track.channels} CH
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
								'flex h-3 w-3 items-center justify-center rounded-full border transition-all duration-200',
								isSelected ? 'border-ds-blue-600' : 'border-gray-alpha-200'
							)}
						>
							<div
								class="h-1.5 w-1.5 rounded-full bg-ds-blue-600 transition-all duration-200"
								style="opacity: {isSelected ? 1 : 0}; transform: scale({isSelected ? 1 : 0.5});"
							></div>
						</div>
					</Button>
				{/each}
			</div>
		</div>
	{/if}
</div>
