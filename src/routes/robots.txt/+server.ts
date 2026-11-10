import type { RequestHandler } from './$types';
import { siteConfig } from '$lib/site/site';

const directives = ['User-agent: *', 'Allow: /'];

const toSitemapUrl = (origin: string) => new URL('/sitemap.xml', origin).href;

export const GET: RequestHandler = () => {
	const canonicalOrigin = new URL(siteConfig.url).origin;
	const lines = [...directives, `Sitemap: ${toSitemapUrl(canonicalOrigin)}`];
	const body = lines.join('\n');

	return new Response(body, {
		headers: {
			'content-type': 'text/plain; charset=utf-8',
			'cache-control': 'public, max-age=3600'
		}
	});
};
