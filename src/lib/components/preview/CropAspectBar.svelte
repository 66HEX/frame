<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import { _ } from '$lib/i18n';
	import { ASPECT_OPTIONS } from '$lib/utils/crop';

	let {
		cropAspect,
		hasCropDimensions,
		onSelectAspect,
		onReset,
		onApply
	}: {
		cropAspect: string;
		hasCropDimensions: boolean;
		onSelectAspect: (id: string) => void;
		onReset: () => void;
		onApply: () => void;
	} = $props();
</script>

<div
	class="button-highlight pointer-events-auto absolute! bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 rounded-md bg-background p-1 text-[10px] shadow-xl"
	onclick={(event) => event.stopPropagation()}
	onpointerdown={(event) => event.stopPropagation()}
	role="presentation"
>
	{#each ASPECT_OPTIONS as option (option.id)}
		<Button
			variant={cropAspect === option.id ? 'default' : 'ghost'}
			onclick={() => onSelectAspect(option.id)}
		>
			{option.labelKey ? $_(option.labelKey) : option.display}
		</Button>
	{/each}
	<div class="h-4 w-px bg-frame-gray-200"></div>
	<Button variant="ghost" onclick={onReset}>{$_('crop.reset')}</Button>
	<Button onclick={onApply} disabled={!hasCropDimensions}>
		{$_('crop.apply')}
	</Button>
</div>
