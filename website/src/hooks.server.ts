import type { Handle } from '@sveltejs/kit/hooks';
import { siteConfig } from '$lib/site/site';
import { contentUiDefaults } from '$lib/site/content-ui';

export const handle: Handle = async ({ event, resolve }) => resolve(event, {
  transformPageChunk: ({ html }) => html
    .replace('%site.language%', siteConfig.language)
    .replace('%site.preferences%', () => JSON.stringify({theme: contentUiDefaults.theme}).replaceAll('<', '\\u003c'))
});
