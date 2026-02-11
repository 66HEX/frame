<script lang="ts">
	import { fly, fade } from 'svelte/transition';
	import { IconClose } from '$lib/icons';
	import Input from '$lib/components/ui/Input.svelte';
	import Label from '$lib/components/ui/Label.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { checkForAppUpdate } from '$lib/services/update';
	import { updateStore } from '$lib/stores/update.svelte';
	import Checkbox from './ui/Checkbox.svelte';
	import Slider from './ui/Slider.svelte';
	import {
		loadAutoUpdateCheck,
		loadWindowOpacity,
		loadFontFamily,
		persistAutoUpdateCheck,
		persistWindowOpacity,
		persistFontFamily
	} from '$lib/services/settings';
	import { themeStore } from '$lib/stores/theme.svelte';
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
	let opacity = $state(themeStore.opacity);
	let fontFamily = $state(themeStore.fontFamily);
	let currentLocale = $state($locale || 'en-US');

	onMount(async () => {
		const [savedAutoUpdateCheck, savedOpacity, savedFontFamily] = await Promise.all([
			loadAutoUpdateCheck(),
			loadWindowOpacity(),
			loadFontFamily()
		]);

		autoUpdateCheck = savedAutoUpdateCheck;
		opacity = savedOpacity;
		fontFamily = savedFontFamily;

		themeStore.opacity = savedOpacity;
		themeStore.fontFamily = savedFontFamily;
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

	$effect(() => {
		if (!hasHydratedSettings) return;
		themeStore.opacity = opacity;
		void persistWindowOpacity(opacity).catch((error) => {
			console.error('Failed to persist window opacity', error);
		});
	});

	$effect(() => {
		if (!hasHydratedSettings) return;
		themeStore.fontFamily = fontFamily;
		void persistFontFamily(fontFamily).catch((error) => {
			console.error('Failed to persist font family', error);
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
		isCheckingForUpdate = false;
		setTimeout(() => {
			checkStatus = '';
		}, 3000);
	}
</script>

<button
	class="absolute inset-0 z-60 cursor-default bg-background/60 backdrop-blur-sm"
	transition:fade={{ duration: 300 }}
	onclick={onClose}
	aria-label="Close settings"
></button>

<div
	class="absolute top-0 right-0 bottom-0 z-70 w-80 rounded-l-xl border-l border-gray-alpha-200 bg-background/60 shadow-2xl backdrop-blur-md"
	transition:fly={{ x: 320, duration: 300, opacity: 1 }}
>
	<div class="flex items-center justify-between border-b border-gray-alpha-100 px-4 py-3">
		<h2 class="text-[10px] font-medium tracking-widest text-foreground">
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
					variant="outline"
				>
					{isSaving ? $_('settings.saving') : $_('common.apply')}
				</Button>
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('settings.visuals')}</Label>
			<div class="space-y-3">
				<div class="flex items-center justify-between">
					<Label for="opacity-slider">{$_('settings.windowTint')}</Label>
					<span class="text-[10px] text-gray-alpha-600">{opacity}%</span>
				</div>
				<Slider id="opacity-slider" min={20} max={100} step={1} bind:value={opacity} />
			</div>

			<div class="space-y-3 pt-2">
				<Label>{$_('settings.fontFamily')}</Label>
				<div class="grid grid-cols-2 gap-2">
					<Button
						variant={fontFamily === 'mono' ? 'selected' : 'outline'}
						onclick={() => (fontFamily = 'mono')}
						class="w-full"
					>
						{$_('settings.fontMono')}
					</Button>
					<Button
						variant={fontFamily === 'sans' ? 'selected' : 'outline'}
						onclick={() => (fontFamily = 'sans')}
						class="w-full"
					>
						{$_('settings.fontSans')}
					</Button>
				</div>
			</div>
		</div>

		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('settings.language')}</Label>
			<div class="flex flex-wrap gap-2">
				{#each supportedLocales as loc (loc.code)}
					<Button
						variant={currentLocale === loc.code ? 'selected' : 'outline'}
						onclick={() => {
							currentLocale = loc.code;
							setLocale(loc.code);
						}}
						size="icon-large"
						class="group relative"
					>
						<span class="text-xl">{loc.flag}</span>
						<span
							class="pointer-events-none absolute -top-8 left-1/2 z-10 -translate-x-1/2 rounded bg-foreground px-2 py-1 text-xs whitespace-nowrap text-background normal-case opacity-0 shadow-lg transition-opacity group-hover:opacity-100"
						>
							{loc.name}
						</span>
					</Button>
				{/each}
			</div>
		</div>
		<div class="space-y-3 pt-2">
			<Label variant="section">{$_('settings.appUpdates')}</Label>
			<div class="flex flex-col space-y-3">
				<div class="flex items-center gap-2 py-0.5">
					<Checkbox id="auto-update-check" bind:checked={autoUpdateCheck} />
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
					<span class="text-[10px] text-blue-600">{checkStatus}</span>
				{/if}
			</div>
		</div>
	</div>
</div>
