import { createRequire } from 'node:module';
import { fileURLToPath, URL } from 'node:url';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { mdsvex } from 'mdsvex';
import { createHighlighter } from 'shiki';
import type { Plugin } from 'vite';
import { defineConfig } from 'vitest/config';
import svelteOptions from './svelte-options.ts';

const themes = {
  light: 'github-light',
  dark: 'github-dark'
};

const highlighter = await createHighlighter({
  themes: Object.values(themes),
  langs: ['svelte', 'bash', 'json', 'typescript', 'tsx', 'vue', 'wgsl', 'css', 'html', 'nix']
});

const markdownLayout = fileURLToPath(
  new URL('./src/lib/features/docs/MarkdownLayout.svelte', import.meta.url)
);

function takumiPrerenderWasm(): Plugin {
  const virtualId = '\0takumi-prerender-wasm';
  const require = createRequire(import.meta.url);
  const imageResponseEntry = require.resolve('@takumi-rs/image-response');
  const takumiJsEntry = createRequire(imageResponseEntry).resolve('takumi-js');
  const wasmPath = createRequire(takumiJsEntry).resolve('@takumi-rs/wasm/takumi_wasm_bg.wasm');

  return {
    name: 'takumi-prerender-wasm',
    apply: 'build',
    enforce: 'pre',
    resolveId(id) {
      if (id === '@takumi-rs/wasm/auto') {
        return virtualId;
      }
      return null;
    },
    load(id) {
      if (id === virtualId) {
        return `import { readFile } from 'node:fs/promises';\nexport default () => readFile(${JSON.stringify(wasmPath)});\n`;
      }
      return null;
    }
  };
}

export default defineConfig({
  plugins: [
    tailwindcss(),
    takumiPrerenderWasm(),
    mdsvex({
      extensions: ['.svx', '.mdx', '.md'],
      heading_ids: true,
      templates: {
        default: markdownLayout
      },
      highlight: (code, { lang, inline }) => {
        if (inline) {
          return null;
        }
        const safeLang = lang || 'text';
        return highlighter.codeToHtml(code, {
          lang: safeLang,
          themes,
          tabindex: false
        });
      }
    }),
    sveltekit(svelteOptions)
  ],
  server: { host: '127.0.0.1' },
  test: { include: ['src/**/*.test.ts'] }
});
