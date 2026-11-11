<script lang="ts">
	import { labels } from '$lib/site/labels';
	import { onDestroy } from 'svelte';
	import { ActionTooltip } from '$lib/components/action-tooltip';
	import { CopyFeedbackIcon } from '$lib/components/copy-feedback';
	import { Button, type ButtonSize } from '$lib/components/ui/button';
	import { CopyIcon } from '$lib/icons';
	import { cn } from "$lib/utils.js";

	type Props = {
		value: string;
		label?: string;
		copiedLabel?: string;
		successMessage?: string;
		failureMessage?: string;
		size?: ButtonSize;
		class?: string;
	};

	let {
		value,
		label = labels.code.copy,
		copiedLabel = labels.code.copied,
		successMessage = labels.code.success,
		failureMessage = labels.code.failure,
		size = 'icon-sm',
		class: className
	}: Props = $props();

	let copied = $state(false);
	let failed = $state(false);
	let announcement = $state('');
	let resetTimer: ReturnType<typeof setTimeout> | undefined;
	let tooltipLabel = $derived(failed ? labels.code.failed : copied ? copiedLabel : label);

	async function copy() {
		try {
			await navigator.clipboard.writeText(value);
			copied = true;
			announcement = successMessage;
			clearTimeout(resetTimer);
			resetTimer = setTimeout(() => {
				copied = false;
				failed = false;
				announcement = '';
			}, 3000);
		} catch {
			copied = false;
			failed = true;
			announcement = failureMessage;
			clearTimeout(resetTimer);
			resetTimer = setTimeout(() => {
				failed = false;
				announcement = '';
			}, 3000);
		}
	}

	onDestroy(() => clearTimeout(resetTimer));
</script>

<ActionTooltip content={tooltipLabel} disableCloseOnTriggerClick>
	{#snippet trigger({ props })}
		<Button
			{...props}
			type="button"
			variant="ghost"
			{size}
			class={cn("text-muted-foreground", className)}
			data-copied={copied}
			aria-label={tooltipLabel}
			onclick={copy}
		>
			<CopyFeedbackIcon {copied} idleIcon={CopyIcon} />
		</Button>
	{/snippet}
</ActionTooltip>
<span class="sr-only" role="status" aria-atomic="true">{announcement}</span>
