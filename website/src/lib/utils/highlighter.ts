import { createHighlighterCoreSync } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import githubLight from 'shiki/themes/github-light.mjs';
import githubDark from 'shiki/themes/github-dark.mjs';

import typescript from 'shiki/langs/typescript.mjs';
import tsx from 'shiki/langs/tsx.mjs';
import svelte from 'shiki/langs/svelte.mjs';
import vue from 'shiki/langs/vue.mjs';
import xml from 'shiki/langs/xml.mjs';
import bash from 'shiki/langs/bash.mjs';
import json from 'shiki/langs/json.mjs';
import wgsl from 'shiki/langs/wgsl.mjs';

let highlighter: ReturnType<typeof createHighlighterCoreSync> | null = null;

export function getHighlighter() {
	highlighter ??= createHighlighterCoreSync({
		themes: [githubLight, githubDark],
		langs: [typescript, tsx, svelte, vue, xml, bash, json, wgsl],
		engine: createJavaScriptRegexEngine()
	});

	return highlighter;
}
