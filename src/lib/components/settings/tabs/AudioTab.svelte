<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, SourceMetadata } from '$lib/types';

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
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Channels & Bitrate
		</span>
		<div class="space-y-3">
			<div class="grid grid-cols-3 gap-2">
				{#each CHANNELS as ch (ch.id)}
					<button
						onclick={() => onUpdate({ audioChannels: ch.id })}
						{disabled}
						class={cn(
							'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
							config.audioChannels === ch.id
								? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
								: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
						)}
					>
						{ch.label}
					</button>
				{/each}
			</div>

			<div class="space-y-2 pt-1">
				<label
					for="audio-bitrate"
					class="text-[10px] text-gray-alpha-600 uppercase tracking-widest whitespace-nowrap"
					>Bitrate (kbps)</label
				>
				<input
					id="audio-bitrate"
					type="number"
					value={config.audioBitrate}
					oninput={(e) => onUpdate({ audioBitrate: e.currentTarget.value })}
					class="w-full text-[11px] uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
					{disabled}
				/>
			</div>
		</div>
	</div>
	<div class="space-y-3">
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Audio Codec
		</span>
		<div class="grid grid-cols-1">
			{#each AUDIO_CODECS as codec (codec.id)}
				{@const isMp3Container = config.container === 'mp3'}
				{@const isAllowed = !isMp3Container || codec.id === 'mp3'}
				<button
					onclick={() => onUpdate({ audioCodec: codec.id })}
					disabled={disabled || !isAllowed}
					class={cn(
						'text-[11px] py-1.5 px-3 border-l-2 text-left transition-all uppercase flex justify-between',
						config.audioCodec === codec.id
							? 'border-l-ds-blue-600 bg-gray-alpha-100 text-foreground pl-3'
							: 'border-l-transparent text-gray-alpha-600 hover:text-foreground pl-2',
						!isAllowed && 'opacity-50 cursor-not-allowed'
					)}
				>
					<span>{codec.id}</span>
					<span class="opacity-50 text-[9px]">
						{isMp3Container && codec.id !== 'mp3' ? 'Incompatible with MP3' : codec.label}
					</span>
				</button>
			{/each}
		</div>
	</div>

	{#if metadata?.audioTracks && metadata.audioTracks.length > 0}
		<div class="space-y-3 pt-1">
			<span
				class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
			>
				Source Tracks
			</span>
			<div class="grid grid-cols-1 gap-2">
				{#each metadata.audioTracks as track (track.index)}
					{@const isSelected = (config.selectedAudioTracks || []).includes(track.index)}
					<button
						onclick={() => toggleTrack(track.index)}
						{disabled}
						class={cn(
							'w-full py-2 px-3 border rounded transition-all text-left flex items-center justify-between',
							isSelected
								? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
								: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
						)}
					>
						<div class="space-y-0.5">
							<div class="flex items-center gap-2">
								<span class="text-[10px] opacity-70">
									#{track.index}
								</span>
								<span class="text-[10px] uppercase font-bold tracking-tight">
									{track.codec}
								</span>
								<div class="text-[9px] uppercase tracking-wide">
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
								'w-4 h-4 rounded-full border flex items-center justify-center transition-all duration-200',
								isSelected ? 'border-ds-blue-600' : 'border-gray-alpha-200'
							)}
						>
							<div
								class="w-2 h-2 rounded-full bg-ds-blue-600 transition-all duration-200"
								style="opacity: {isSelected ? 1 : 0}; transform: scale({isSelected ? 1 : 0.5});"
							></div>
						</div>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
