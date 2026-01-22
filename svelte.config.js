import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			fallback: 'index.html'
		})
	},
	vitePlugin: {
		inspector: {
			toggleKeyCombo: 'alt-x',
			holdMode: true,
			showOutputWindow: false
		}
	}
};

export default config;
