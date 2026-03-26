<script lang="ts">
	import { fly, fade } from 'svelte/transition';
	import { IconClose } from '$lib/icons';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';
	import { checkForAppUpdate } from '$lib/services/update';
	import { updateStore } from '$lib/stores/update.svelte';
	import Checkbox from './ui/Checkbox.svelte';
	import { loadAutoUpdateCheck, persistAutoUpdateCheck } from '$lib/services/settings';
	import { onMount } from 'svelte';
	import { _, locale, setLocale, supportedLocales } from '$lib/i18n';

	let {
		maxConcurrency,
		onUpdate,
		onClose
	}: {
		maxConcurrency: number;
		onUpdate: (value: number) => void | Promise<void>;
		onClose: () => void;
	} = $props();

	let localValue = $derived.by(() => {
		let value = $state(String(maxConcurrency));
		return {
			get current() {
				return value;
			},
			set current(v) {
				value = v;
			}
		};
	});

	let isSaving = $state(false);
	let isCheckingForUpdate = $state(false);
	let hasHydratedSettings = $state(false);
	let checkStatus = $state('');
	let autoUpdateCheck = $state(true);
	let currentLocale = $state($locale || 'en-US');

	onMount(async () => {
		autoUpdateCheck = await loadAutoUpdateCheck();
		hasHydratedSettings = true;
	});

	$effect(() => {
		if ($locale) {
			currentLocale = $locale;
		}
	});

	$effect(() => {
		if (!hasHydratedSettings) return;
		void persistAutoUpdateCheck(autoUpdateCheck).catch((error) => {
			console.error('Failed to persist auto-update setting', error);
		});
	});

	async function handleSave() {
		const parsed = Number(localValue.current);
		isSaving = true;
		try {
			await onUpdate(parsed);
		} finally {
			isSaving = false;
		}
	}

	async function handleCheckUpdate() {
		isCheckingForUpdate = true;
		checkStatus = '';
		try {
			const result = await checkForAppUpdate();
			if (result.available) {
				updateStore.isAvailable = true;
				updateStore.version = result.version || '';
				updateStore.body = result.body || '';
				updateStore.updateObject = result.updateObject;
				updateStore.showDialog = true;
				checkStatus = $_('settings.updateAvailable');
			} else {
				checkStatus = $_('settings.latestVersion');
			}
		} catch (error) {
			console.error('Manual update check failed', error);
			checkStatus = $_('settings.errorChecking');
		} finally {
			isCheckingForUpdate = false;
		}

		setTimeout(() => {
			checkStatus = '';
		}, 3000);
	}
</script>

<button
	class="absolute inset-0 z-60 bg-background/60 backdrop-blur-sm"
	transition:fade={{ duration: 300 }}
	onclick={onClose}
	aria-label="Close settings"
></button>

<div
	class="absolute top-0 right-0 bottom-0 z-70 w-80 rounded-l-xl border-l border-gray-alpha-200 bg-sidebar shadow-2xl"
	transition:fly={{ x: 320, duration: 300, opacity: 1 }}
>
	<div class="flex items-center justify-between border-b border-gray-alpha-100 px-4 py-3">
		<h2 class="text-[10px] font-semibold text-foreground">
			{$_('settings.title')}
		</h2>
		<button onclick={onClose} class="text-gray-alpha-600 transition-colors hover:text-foreground">
			<IconClose size={16} />
		</button>
	</div>

	<div class="space-y-4 p-4">
		<div class="space-y-3">
			<Label for="max-concurrency" variant="section">{$_('settings.maxConcurrency')}</Label>
			<div class="flex items-center gap-2">
				<div class="flex-1">
					<Input
						id="max-concurrency"
						type="text"
						inputmode="numeric"
						value={localValue.current}
						oninput={(e) => {
							const sanitized = e.currentTarget.value.replace(/[^0-9]/g, '');
							if (sanitized !== e.currentTarget.value) {
								e.currentTarget.value = sanitized;
							}
							localValue.current = sanitized;
						}}
						placeholder="2"
						disabled={isSaving}
					/>
				</div>
				<Button
					onclick={handleSave}
					disabled={isSaving || localValue.current === String(maxConcurrency)}
					variant="secondary"
				>
					{isSaving ? $_('settings.saving') : $_('common.apply')}
				</Button>
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('settings.language')}</Label>
			<div class="flex flex-wrap gap-2">
				{#each supportedLocales as loc (loc.code)}
					<Tooltip content={loc.name}>
						<Button
							variant={currentLocale === loc.code ? 'default' : 'secondary'}
							onclick={() => {
								currentLocale = loc.code;
								setLocale(loc.code);
							}}
							size="icon"
						>
							<span class="emoji text-base">{loc.flag}</span>
						</Button>
					</Tooltip>
				{/each}
			</div>
		</div>
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('settings.appUpdates')}</Label>
			<div class="flex flex-col space-y-3">
				<div class="flex items-center gap-2 py-0.5">
					<Checkbox class="mt-px" id="auto-update-check" bind:checked={autoUpdateCheck} />
					<Label for="auto-update-check">{$_('settings.checkOnStartup')}</Label>
				</div>
				<Button
					variant="default"
					class="w-full"
					onclick={handleCheckUpdate}
					disabled={isCheckingForUpdate}
				>
					{isCheckingForUpdate ? $_('settings.checking') : $_('settings.checkForUpdates')}
				</Button>
				{#if checkStatus}
					<span class="text-[10px] font-semibold text-blue-700">{checkStatus}</span>
				{/if}
			</div>
		</div>
	</div>
</div>
