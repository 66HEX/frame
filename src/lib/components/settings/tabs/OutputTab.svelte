<script lang="ts">
	import {
		ALL_CONTAINERS,
		AUDIO_ONLY_CONTAINERS,
		type ConversionConfig,
		type SourceMetadata
	} from '$lib/types';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import { _ } from '$lib/i18n';

	import { isAudioCodecAllowed, getDefaultAudioCodec } from '$lib/services/media';

	let {
		config,
		disabled = false,
		outputName = '',
		metadata,
		onUpdate,
		onUpdateOutputName
	}: {
		config: ConversionConfig;
		disabled?: boolean;
		outputName?: string;
		metadata?: SourceMetadata;
		onUpdate: (config: Partial<ConversionConfig>) => void;
		onUpdateOutputName?: (value: string) => void;
	} = $props();

	const isSourceAudioOnly = $derived(!!metadata && !metadata.videoCodec);

	function sanitizeOutputName(value: string): string {
		const candidate = value.split(/[/\\]/).pop()?.trim() ?? '';
		return candidate === '.' || candidate === '..' ? '' : candidate;
	}

	function handleContainerChange(newContainer: string) {
		const updates: Partial<ConversionConfig> = { container: newContainer };

		if (!isAudioCodecAllowed(config.audioCodec, newContainer)) {
			updates.audioCodec = getDefaultAudioCodec(newContainer);
		}

		onUpdate(updates);
	}
</script>

<div class="space-y-4">
	<div class="space-y-3">
		<Label variant="section">{$_('output.outputName')}</Label>
		<Input
			type="text"
			value={outputName}
			oninput={(e) => onUpdateOutputName?.(sanitizeOutputName(e.currentTarget.value))}
			placeholder={$_('output.placeholder')}
			{disabled}
		/>
		<p class="text-[9px] tracking-wide text-gray-alpha-600">
			{$_('output.hint')}
		</p>
	</div>

	<div class="space-y-3 pt-2">
		<Label variant="section">{$_('output.container')}</Label>
		<div class="grid grid-cols-2 gap-2">
			{#each ALL_CONTAINERS as fmt (fmt)}
				{@const isVideoContainer = !AUDIO_ONLY_CONTAINERS.includes(fmt)}
				{@const isDisabled = disabled || (isSourceAudioOnly && isVideoContainer)}
				<Button
					variant={config.container === fmt ? 'selected' : 'outline'}
					onclick={() => handleContainerChange(fmt)}
					disabled={isDisabled}
					class="w-full"
				>
					{fmt}
				</Button>
			{/each}
		</div>
	</div>
</div>
