<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig } from '$lib/types';

	const RESOLUTIONS = ['original', '1080p', '720p', '480p', 'custom'] as const;
	const VIDEO_CODECS = [
		{ id: 'libx264', label: 'H.264 / AVC' },
		{ id: 'libx265', label: 'H.265 / HEVC' },
		{ id: 'vp9', label: 'VP9 / Web' },
		{ id: 'prores', label: 'Apple ProRes' },
		{ id: 'libsvtav1', label: 'AV1 / SVT' },
		{ id: 'h264_videotoolbox', label: 'H.264 (Apple Silicon)' },
		{ id: 'h264_nvenc', label: 'H.264 (NVIDIA)' }
	] as const;

	const SCALING_ALGOS = [
		{ id: 'bicubic', label: 'Bicubic' },
		{ id: 'lanczos', label: 'Lanczos' },
		{ id: 'bilinear', label: 'Bilinear' },
		{ id: 'nearest', label: 'Nearest' }
	] as const;

	const FPS_OPTIONS = [
		{ id: 'original', label: 'Same as source' },
		{ id: '24', label: '24 fps' },
		{ id: '30', label: '30 fps' },
		{ id: '60', label: '60 fps' }
	] as const;

	let {
		config,
		disabled = false,
		onUpdate
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		onUpdate: (config: Partial<ConversionConfig>) => void;
	} = $props();
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Resolution & Framerate
		</span>
		<div class="grid grid-cols-2 gap-2 mb-2">
			{#each RESOLUTIONS as res (res)}
				<button
					onclick={() => onUpdate({ resolution: res })}
					{disabled}
					class={cn(
						'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
						config.resolution === res
							? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
							: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
					)}
				>
					{res}
				</button>
			{/each}
		</div>

		{#if config.resolution === 'custom'}
			<div class="grid grid-cols-2 gap-2 mb-2 pt-1">
				<div class="flex flex-col gap-1">
					<label for="width" class="text-[9px] text-gray-alpha-600 uppercase tracking-widest pb-1"
						>Width</label
					>
					<input
						id="width"
						type="number"
						placeholder="1920"
						value={config.customWidth}
						oninput={(e) => onUpdate({ customWidth: e.currentTarget.value })}
						class="w-full text-[11px] uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
						{disabled}
					/>
				</div>
				<div class="flex flex-col gap-1">
					<label for="height" class="text-[9px] text-gray-alpha-600 uppercase tracking-widest pb-1"
						>Height</label
					>
					<input
						id="height"
						type="number"
						placeholder="1080"
						value={config.customHeight}
						oninput={(e) => onUpdate({ customHeight: e.currentTarget.value })}
						class="w-full text-[11px] uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
						{disabled}
					/>
				</div>
			</div>
		{/if}

		<div class="space-y-3 pt-2">
			<span
				class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
			>
				Scaling Algorithm
			</span>
			<div class="grid grid-cols-2 gap-2">
				{#each SCALING_ALGOS as algo (algo.id)}
					<button
						onclick={() => onUpdate({ scalingAlgorithm: algo.id })}
						disabled={disabled || config.resolution === 'original'}
						class={cn(
							'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
							config.scalingAlgorithm === algo.id
								? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
								: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 disabled:cursor-not-allowed hover:bg-gray-alpha-100 hover:text-foreground disabled:opacity-50'
						)}
					>
						{algo.label}
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<span
				class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
			>
				Framerate
			</span>
			<div class="grid grid-cols-2 gap-2">
				{#each FPS_OPTIONS as opt (opt.id)}
					<button
						onclick={() => onUpdate({ fps: opt.id })}
						{disabled}
						class={cn(
							'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
							config.fps === opt.id
								? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
								: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
						)}
					>
						{opt.label}
					</button>
				{/each}
			</div>
		</div>
	</div>

	<div class="space-y-3 pt-2">
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Video Encoder
		</span>
		<div class="grid grid-cols-1 gap-1.5">
			{#each VIDEO_CODECS as codec (codec.id)}
				<button
					onclick={() => onUpdate({ videoCodec: codec.id })}
					{disabled}
					class={cn(
						'text-[11px] py-1.5 px-3 border-l-2 text-left transition-all uppercase flex justify-between',
						config.videoCodec === codec.id
							? 'border-l-ds-blue-600 bg-gray-alpha-100 text-foreground pl-3'
							: 'border-l-transparent text-gray-alpha-600 hover:text-foreground pl-2'
					)}
				>
					<span>{codec.id}</span>
					<span class="opacity-50 text-[9px]">{codec.label}</span>
				</button>
			{/each}
		</div>
	</div>

	<div class="space-y-3 pt-2">
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Quality Control
		</span>
		<div class="grid grid-cols-2 gap-2">
			<button
				onclick={() => onUpdate({ videoBitrateMode: 'crf' })}
				{disabled}
				class={cn(
					'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
					config.videoBitrateMode === 'crf'
						? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
						: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
				)}
			>
				Constant Quality
			</button>
			<button
				onclick={() => onUpdate({ videoBitrateMode: 'bitrate' })}
				{disabled}
				class={cn(
					'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
					config.videoBitrateMode === 'bitrate'
						? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
						: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
				)}
			>
				Target Bitrate
			</button>
		</div>
	</div>

	{#if config.videoBitrateMode === 'crf'}
		<div class="space-y-2 pt-2">
			<div class="flex justify-between items-end">
				<label
					for="quality-factor"
					class="text-[10px] text-gray-alpha-600 uppercase tracking-widest pb-1"
					>Quality Factor</label
				>
				<div
					class="text-[10px] bg-ds-blue-900/20 text-ds-blue-600 border border-ds-blue-600 px-1.5 py-1 rounded font-medium"
				>
					CRF {config.crf}
				</div>
			</div>
			<div class="h-1.5 bg-gray-alpha-100 rounded-full w-full relative">
				<div
					class="absolute top-0 left-0 h-full bg-foreground rounded-full"
					style="width: {(config.crf / 51) * 100}%"
				></div>
				<input
					id="quality-factor"
					type="range"
					min="0"
					max="51"
					value={config.crf}
					oninput={(e) => onUpdate({ crf: parseInt(e.currentTarget.value) })}
					class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
					{disabled}
				/>
			</div>
			<div class="flex justify-between text-[9px] text-gray-alpha-600 uppercase">
				<span>Lossless</span>
				<span>Smallest</span>
			</div>
		</div>
	{:else}
		<div class="space-y-2 pt-1">
			<div class="flex justify-between items-end">
				<label
					for="video-bitrate"
					class="text-[10px] text-gray-alpha-600 uppercase tracking-widest whitespace-nowrap"
					>Target Bitrate (kbps)</label
				>
			</div>
			<div class="flex items-center gap-2">
				<input
					id="video-bitrate"
					type="number"
					value={config.videoBitrate}
					oninput={(e) => onUpdate({ videoBitrate: e.currentTarget.value })}
					class="w-full text-[11px] uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
					{disabled}
				/>
			</div>
		</div>
	{/if}
</div>
