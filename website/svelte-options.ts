import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { type Config } from '@sveltejs/kit/vite';

const config: Config = {
	extensions: ['.svelte', '.svx', '.mdx', '.md'],
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: [vitePreprocess()],

	adapter: adapter(),
	alias: { $lib: './src/lib' }
};

export default config;
