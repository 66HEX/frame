<script lang="ts">
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig } from '$lib/types';

	const CONTAINERS = ['mp4', 'mkv', 'webm', 'mov', 'mp3'] as const;

	let {
		config,
		disabled = false,
		outputName = '',
		onUpdate,
		onUpdateOutputName
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		outputName?: string;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		onUpdateOutputName?: (value: string) => void;
	} = $props();
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<div
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest border-b border-gray-alpha-100 pb-1"
		>
			Output Name
		</div>
		<input
			type="text"
			value={outputName}
			oninput={(e) => onUpdateOutputName?.(e.currentTarget.value)}
			placeholder="my_render_final"
			class="w-full text-[11px] placeholder:uppercase tracking-wide px-3 py-1.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all"
			{disabled}
		/>
		<p class="text-[10px] text-gray-alpha-600 uppercase tracking-wide">
			Stored next to the original file. Extension follows the selected container automatically.
		</p>
	</div>

	<div class="space-y-3 pt-2">
		<span
			class="text-[10px] text-gray-alpha-600 uppercase tracking-widest block border-b border-gray-alpha-100 pb-1"
		>
			Output Container
		</span>
		<div class="grid grid-cols-2 gap-2">
			{#each CONTAINERS as fmt (fmt)}
				<button
					onclick={() => onUpdate({ container: fmt })}
					{disabled}
					class={cn(
						'text-[11px] py-1.5 px-2 border rounded transition-all text-center uppercase',
						config.container === fmt
							? 'bg-ds-blue-900/20 text-ds-blue-600 border-ds-blue-600'
							: 'bg-transparent text-gray-alpha-600 border-gray-alpha-200 hover:bg-gray-alpha-100 hover:text-foreground'
					)}
				>
					{fmt}
				</button>
			{/each}
		</div>
	</div>
</div>
