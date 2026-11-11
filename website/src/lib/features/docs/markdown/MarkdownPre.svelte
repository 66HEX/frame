<script lang="ts">
	import { CodeBlock, CopyCodeButton } from '$lib/components/code-block';
	import * as Card from '$lib/components/ui/card';
	import {
		FrameworkReactIcon,
		FrameworkSvelteIcon,
		FrameworkVueIcon,
		LanguageBashIcon,
		LanguageCssIcon,
		LanguageDotenvIcon,
		LanguageHtmlIcon,
		LanguageJavaScriptIcon,
		LanguageJsonIcon,
		LanguageMarkdownIcon,
		LanguageTypeScriptIcon,
		LanguageWebGlIcon,
		LanguageWebGpuIcon,
		type IconComponent
	} from '$lib/icons';

	let {
		htmlLight,
		htmlDark,
		lang,
		label,
		raw
	}: {
		htmlLight: string;
		htmlDark?: string | undefined;
		lang?: string | undefined;
		label?: string | undefined;
		raw?: string;
	} = $props();

	const language = $derived((lang ?? 'text').toLowerCase());
	const fileName = $derived(inferFileName(language));
	const code = $derived(raw ?? '');

	const languageIcons: Record<string, IconComponent> = {
		svelte: FrameworkSvelteIcon,
		react: FrameworkReactIcon,
		jsx: FrameworkReactIcon,
		tsx: FrameworkReactIcon,
		vue: FrameworkVueIcon,
		typescript: LanguageTypeScriptIcon,
		ts: LanguageTypeScriptIcon,
		javascript: LanguageJavaScriptIcon,
		js: LanguageJavaScriptIcon,
		css: LanguageCssIcon,
		html: LanguageHtmlIcon,
		json: LanguageJsonIcon,
		dotenv: LanguageDotenvIcon,
		env: LanguageDotenvIcon,
		wgsl: LanguageWebGpuIcon,
		glsl: LanguageWebGlIcon,
		markdown: LanguageMarkdownIcon,
		md: LanguageMarkdownIcon,
		bash: LanguageBashIcon,
		shell: LanguageBashIcon,
		sh: LanguageBashIcon
	};
	const LanguageIcon = $derived(languageIcons[language]);

	function inferFileName(value: string) {
		if (value === 'svelte') return 'Example.svelte';
		if (value === 'react' || value === 'jsx' || value === 'tsx') return 'Example.tsx';
		if (value === 'vue') return 'Example.vue';
		if (value === 'typescript' || value === 'ts') return 'example.ts';
		if (value === 'javascript' || value === 'js') return 'example.js';
		if (value === 'css') return 'styles.css';
		if (value === 'html') return 'index.html';
		if (value === 'json') return 'components.json';
		if (value === 'dotenv' || value === 'env') return '.env.local';
		if (value === 'wgsl') return 'shader.wgsl';
		if (value === 'glsl') return 'shader.glsl';
		if (value === 'markdown' || value === 'md') return 'README.md';
		if (value === 'bash' || value === 'shell' || value === 'sh') return 'Terminal';
		return 'Example';
	}
</script>

<Card.Root size="sm" class="my-6 gap-0 bg-muted px-1 pt-0 pb-1 transition-shadow has-[[data-scrollable]:focus-visible]:ring-ring/50 has-[[data-scrollable]:focus-visible]:ring-[3px] has-[[data-scrollable]:focus-visible]:outline-1">
	<div class="relative flex h-11 items-center px-4">
		{#if LanguageIcon}
			<span
				class="relative me-2 flex size-4 shrink-0 items-center justify-center text-muted-foreground"
				aria-hidden="true"
			>
				<LanguageIcon size={16} />
			</span>
		{/if}

		<span class="min-w-0 truncate pe-10 text-sm font-medium text-foreground">
			{fileName}
		</span>

		<CopyCodeButton
			value={code}
			label="Copy code to clipboard"
			size="icon-sm"
			class="absolute right-2"
		/>
	</div>

	<div class="rounded-lg bg-card shadow-sm">
		<CodeBlock
			{code}
			{htmlLight}
			{htmlDark}
			label={label ?? `${language} code example`}
			framed={false}
			copyable={false}
			class="bg-transparent"
		/>
	</div>
</Card.Root>
