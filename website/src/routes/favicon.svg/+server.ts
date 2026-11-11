import { siteConfig } from '$lib/site/site';
export const prerender = true;
export function GET() {
	const svg = siteConfig.logoRaw
		.replace('<svg ', '<svg style="color: #18181b" ')
		.replace(
			'</svg>',
			'<style>@media(prefers-color-scheme:dark){svg{color:#fafafa!important}}</style></svg>'
		);
	return new Response(svg, {
		headers: { 'Content-Type': 'image/svg+xml', 'Cache-Control': 'public, max-age=3600' }
	});
}
