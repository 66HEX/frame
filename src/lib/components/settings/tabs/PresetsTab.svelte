<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Trash2 } from 'lucide-svelte';
	import { cn } from '$lib/utils/cn';
	import type { ConversionConfig, PresetDefinition } from '$lib/types';

	let {
		config,
		presets = [],
		disabled = false,
		onApplyPreset,
		onSavePreset,
		onDeletePreset
	}: {
		config: ConversionConfig;
		presets?: PresetDefinition[];
		disabled?: boolean;
		onApplyPreset?: (preset: PresetDefinition) => void;
		onSavePreset?: (name: string) => Promise<boolean | void> | boolean | void;
		onDeletePreset?: (id: string) => Promise<boolean | void> | boolean | void;
	} = $props();

	let newPresetName = $state('');
	type NoticeTone = 'success' | 'error';
	let notice = $state<{ text: string; tone: NoticeTone } | null>(null);
	let noticeTimeout: ReturnType<typeof setTimeout> | null = null;

	onDestroy(() => {
		if (noticeTimeout) clearTimeout(noticeTimeout);
	});

	function configsMatch(a: ConversionConfig, b: ConversionConfig) {
		return (
			a.container === b.container &&
			a.videoCodec === b.videoCodec &&
			a.audioCodec === b.audioCodec &&
			a.resolution === b.resolution &&
			a.crf === b.crf &&
			a.preset === b.preset
		);
	}

	function showNotice(text: string, tone: NoticeTone = 'success') {
		notice = { text, tone };
		if (noticeTimeout) clearTimeout(noticeTimeout);
		noticeTimeout = setTimeout(() => (notice = null), 2400);
	}

	async function savePreset() {
		if (!onSavePreset || disabled) return;
		if (!newPresetName.trim()) {
			showNotice('Name required', 'error');
			return;
		}

		const result = await onSavePreset(newPresetName.trim());
		if (result === false) {
			showNotice('Preset not saved', 'error');
			return;
		}

		newPresetName = '';
		showNotice('Preset saved');
	}

	function applyPreset(preset: PresetDefinition) {
		if (disabled) return;
		onApplyPreset?.(preset);
		showNotice(`Applied ${preset.name}`);
	}

	async function removePreset(preset: PresetDefinition) {
		if (!onDeletePreset || preset.builtIn) return;
		const result = await onDeletePreset(preset.id);
		if (result === false) {
			showNotice('Unable to delete', 'error');
			return;
		}

		showNotice('Preset removed');
	}
</script>

<div class="space-y-3">
	<div class="flex items-center justify-between border-b border-gray-alpha-100 pb-1">
		<span class="text-[10px] text-gray-alpha-600 uppercase tracking-widest">Preset Library</span>
		{#if notice}
			<span
				class={cn(
					'text-[9px]  uppercase tracking-wide',
					notice.tone === 'error' ? 'text-ds-red-700' : 'text-ds-blue-600'
				)}
			>
				{notice.text}
			</span>
		{/if}
	</div>

	<div class="flex gap-2">
		<input
			type="text"
			bind:value={newPresetName}
			placeholder="Preset Label"
			class="flex-1 text-[11px] placeholder:uppercase tracking-wide px-3 py-1.5 h-7.5 border border-gray-alpha-200 rounded bg-transparent focus:outline-none focus:border-ds-blue-600! transition-all"
			{disabled}
		/>
		<button
			onclick={savePreset}
			disabled={disabled || !newPresetName.trim()}
			class={cn(
				'px-3 py-1.5 text-[10px] h-7.5 uppercase tracking-wide border rounded transition-all',
				disabled || !newPresetName.trim()
					? 'opacity-50 cursor-not-allowed border-gray-alpha-200 text-gray-alpha-600'
					: 'border-ds-blue-600 text-ds-blue-600 hover:bg-ds-blue-900/20'
			)}
		>
			Save
		</button>
	</div>

	<div class="space-y-1.5 max-h-52 overflow-y-auto">
		{#each presets as preset (preset.id)}
			<div
				class={cn(
					'w-full flex items-center gap-2 border rounded px-2 py-1.5 h-7.5 transition-all text-left cursor-pointer',
					configsMatch(config, preset.config)
						? 'bg-ds-blue-900/20 border-ds-blue-600 text-ds-blue-600'
						: 'border-gray-alpha-200 hover:bg-gray-alpha-100 text-gray-alpha-600 hover:text-foreground!'
				)}
				role="button"
				tabindex="0"
				onclick={() => applyPreset(preset)}
				onkeydown={(event) => {
					if (event.key === 'Enter' || event.key === ' ') {
						event.preventDefault();
						applyPreset(preset);
					}
				}}
			>
				<span
					class="flex-1 flex items-center justify-between gap-2 text-[11px] uppercase tracking-tight pointer-events-none"
				>
					<span class="truncate">{preset.name}</span>
					<span class="text-[9px] font-semibold">
						{configsMatch(config, preset.config) ? 'APPLIED' : 'APPLY'}
					</span>
				</span>
				{#if !preset.builtIn}
					<button
						type="button"
						class="size-4 flex items-center justify-center rounded text-gray-alpha-600 hover:text-ds-red-600 transition-colors"
						title="Delete preset"
						onclick={(event) => {
							event.stopPropagation();
							removePreset(preset);
						}}
						{disabled}
					>
						<Trash2 size={12} />
					</button>
				{/if}
			</div>
		{/each}
	</div>
</div>
