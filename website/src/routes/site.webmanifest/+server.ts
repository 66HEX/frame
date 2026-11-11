import { siteConfig } from '$lib/site/site';
export const prerender = true;
export function GET() {
	return new Response(
		JSON.stringify({
			name: siteConfig.name,
			short_name: siteConfig.shortName,
			description: siteConfig.description,
			lang: siteConfig.language,
			start_url: siteConfig.home.href,
			icons: siteConfig.assets.manifestIcons,
			theme_color: siteConfig.themeColor.light,
			background_color: siteConfig.themeColor.light,
			display: 'standalone'
		}),
		{ headers: { 'Content-Type': 'application/manifest+json' } }
	);
}
