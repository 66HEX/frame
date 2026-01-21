<script lang="ts">
	import type { ConversionConfig } from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';

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
		<Label variant="section">Output Name</Label>
		<Input
			type="text"
			value={outputName}
			oninput={(e) => onUpdateOutputName?.(e.currentTarget.value)}
			placeholder="my_render_final"
			class="placeholder:uppercase"
			{disabled}
		/>
		<p class="text-gray-alpha-600 text-[10px] tracking-wide uppercase">
			Stored next to the original file. Extension follows the selected container automatically.
		</p>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">Output Container</Label>
		<div class="grid grid-cols-2 gap-2">
			{#each CONTAINERS as fmt (fmt)}
				<Button
					variant={config.container === fmt ? 'selected' : 'outline'}
					onclick={() => onUpdate({ container: fmt })}
					{disabled}
					class="w-full"
				>
					{fmt}
				</Button>
			{/each}
		</div>
	</div>
</div>
