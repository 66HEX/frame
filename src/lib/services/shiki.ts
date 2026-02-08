import { createHighlighterCore, type HighlighterCore } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import ffmpegLogGrammar from './ffmpeg-log.tmLanguage.json';

let highlighterPromise: Promise<HighlighterCore> | null = null;

/**
 * Custom FFmpeg log language definition for Shiki.
 * Highlights phase prefixes, timestamps, paths, codecs, and errors.
 */
const ffmpegLogLang = ffmpegLogGrammar;

/**
 * Get or create the singleton highlighter instance.
 * Uses fine-grained bundle with JavaScript engine for optimal web performance.
 */
export function getHighlighter(): Promise<HighlighterCore> {
	if (!highlighterPromise) {
		highlighterPromise = createHighlighterCore({
			themes: [import('@shikijs/themes/github-dark')],
			langs: [ffmpegLogLang],
			engine: createJavaScriptRegexEngine()
		});
	}
	return highlighterPromise;
}

/**
 * Highlight a single line of log text.
 * Returns HTML string with syntax highlighting.
 */
export async function highlightLogLine(line: string): Promise<string> {
	const highlighter = await getHighlighter();
	return highlighter.codeToHtml(line, {
		lang: 'ffmpeg-log',
		theme: 'github-dark'
	});
}

/**
 * Synchronous version - requires highlighter to be pre-loaded.
 * Use this for performance-critical rendering in virtualized lists.
 */
export function highlightLogLineSync(highlighter: HighlighterCore, line: string): string {
	return highlighter.codeToHtml(line, {
		lang: 'ffmpeg-log',
		theme: 'github-dark'
	});
}
