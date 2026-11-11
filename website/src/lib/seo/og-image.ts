import ImageResponse from '@takumi-rs/image-response';
import overusedGroteskRegularDataUri from '$lib/assets/fonts/overused-grotesk/OverusedGroteskRoman-Roman.woff2?inline';
import overusedGroteskMediumDataUri from '$lib/assets/fonts/overused-grotesk/OverusedGroteskRoman-Medium.woff2?inline';
import { siteConfig } from '$lib/site/site';
import { ogThemeColors, toSvgColor, withAlpha } from '$lib/seo/og-theme';

const OG_WIDTH = 1200;
const OG_HEIGHT = 630;
const OG_GRID_INSET = 72;

const DIVIDER_DASH_LENGTH = 6;
const DIVIDER_DASH_GAP = 6;
const VERTICAL_DASH_COUNT = Math.ceil(OG_HEIGHT / (DIVIDER_DASH_LENGTH + DIVIDER_DASH_GAP));
const HORIZONTAL_DASH_COUNT = Math.ceil(OG_WIDTH / (DIVIDER_DASH_LENGTH + DIVIDER_DASH_GAP));

type TakumiElement = {
	type: string;
	props: Record<string, unknown>;
	key: string | null;
};

type TakumiChild = TakumiElement | string;

export type OgImageOptions = {
	title: string;
	description: string;
	titleFontSize?: number;
	titleLineHeight?: number;
	descriptionLineHeight?: number;
};

const el = (
	type: string,
	props: Record<string, unknown> = {},
	...children: TakumiChild[]
): TakumiElement => ({
	type,
	key: null,
	props:
		children.length === 0
			? props
			: {
					...props,
					children: children.length === 1 ? children[0] : children
				}
});

const dataUriToArrayBuffer = (dataUri: string) => {
	const base64 = dataUri.slice(dataUri.indexOf(',') + 1);

	if (typeof Buffer !== 'undefined') {
		const bytes = Buffer.from(base64, 'base64');
		return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
	}

	const binary = atob(base64);
	const bytes = new Uint8Array(binary.length);
	for (let index = 0; index < binary.length; index += 1) {
		bytes[index] = binary.charCodeAt(index);
	}
	return bytes.buffer;
};

const fontDataPromise = Promise.all([
	Promise.resolve(dataUriToArrayBuffer(overusedGroteskRegularDataUri)),
	Promise.resolve(dataUriToArrayBuffer(overusedGroteskMediumDataUri))
]);

const takumiFontLoaders = [
	{
		key: 'overused-grotesk-regular',
		name: 'Overused Grotesk',
		weight: 400,
		style: 'normal' as const,
		data: async () => (await fontDataPromise)[0]
	},
	{
		key: 'overused-grotesk-medium',
		name: 'Overused Grotesk',
		weight: 500,
		style: 'normal' as const,
		data: async () => (await fontDataPromise)[1]
	}
];

const colors = {
	background: toSvgColor(ogThemeColors.dark.background),
	foreground: toSvgColor(ogThemeColors.dark.foreground),
	accent: toSvgColor(ogThemeColors.dark.accent),
	guide: withAlpha(ogThemeColors.dark.shadowHighlight, 0.1)
};

const logoDataUri = `data:image/svg+xml,${encodeURIComponent(
	siteConfig.logoRaw.replaceAll('currentColor', withAlpha(colors.foreground, 0.58))
)}`;

const verticalDivider = (left: number) =>
	el(
		'div',
		{
			style: {
				position: 'absolute',
				top: 0,
				bottom: 0,
				left,
				display: 'flex',
				flexDirection: 'column',
				gap: DIVIDER_DASH_GAP,
				width: 1,
				overflow: 'hidden'
			}
		},
		...Array.from({ length: VERTICAL_DASH_COUNT }, () =>
			el('div', {
				style: {
					display: 'flex',
					width: 1,
					height: DIVIDER_DASH_LENGTH,
					flexShrink: 0,
					background: colors.guide
				}
			})
		)
	);

const horizontalDivider = (top: number) =>
	el(
		'div',
		{
			style: {
				position: 'absolute',
				top,
				right: 0,
				left: 0,
				display: 'flex',
				gap: DIVIDER_DASH_GAP,
				height: 1,
				overflow: 'hidden'
			}
		},
		...Array.from({ length: HORIZONTAL_DASH_COUNT }, () =>
			el('div', {
				style: {
					display: 'flex',
					width: DIVIDER_DASH_LENGTH,
					height: 1,
					flexShrink: 0,
					background: colors.guide
				}
			})
		)
	);

const createComponent = ({
	title,
	description,
	titleFontSize = 64,
	titleLineHeight = 1.02,
	descriptionLineHeight = 1.3
}: OgImageOptions) =>
	el(
		'div',
		{
			style: {
				position: 'relative',
				display: 'flex',
				alignItems: 'center',
				justifyContent: 'center',
				width: '100%',
				height: '100%',
				overflow: 'hidden',
				backgroundColor: colors.background,
				color: colors.foreground,
				fontFamily: 'Overused Grotesk, sans-serif'
			}
		},
		el('div', {
			style: {
				position: 'absolute',
				top: -90,
				right: -90,
				bottom: -90,
				left: -90,
				backgroundColor: colors.background,
				backgroundImage: `radial-gradient(ellipse 50% 80% at 50% 50%, rgba(0, 0, 0, 0) 72%, rgba(0, 0, 0, 0.92) 100%), radial-gradient(ellipse 100% 110% at 50% 0%, ${colors.background} 37%, ${colors.accent} 69%, ${colors.foreground} 100%)`,
				backgroundPosition: 'center',
				backgroundRepeat: 'no-repeat',
				backgroundSize: `${OG_WIDTH.toString()}px ${OG_HEIGHT.toString()}px`,
				filter: 'blur(90px)'
			}
		}),
		verticalDivider(OG_GRID_INSET),
		verticalDivider(OG_WIDTH - OG_GRID_INSET),
		horizontalDivider(OG_GRID_INSET),
		horizontalDivider(OG_HEIGHT - OG_GRID_INSET),
		el(
			'div',
			{
				style: {
					position: 'absolute',
					bottom: OG_GRID_INSET + 60,
					right: OG_GRID_INSET + 12,
					display: 'flex',
					alignItems: 'center',
					gap: 14,
					height: 28
				}
			},
			el('img', {
				src: logoDataUri,
				alt: '',
				style: {
					display: 'flex',
					width: 120,
					height: 120
				}
			})
		),
		el(
			'div',
			{
				style: {
					position: 'absolute',
					bottom: OG_GRID_INSET + 24,
					left: OG_GRID_INSET + 18,
					display: 'flex',
					flexDirection: 'column',
					alignItems: 'flex-start',
					gap: 18
				}
			},
			el(
				'div',
				{
					style: {
						display: 'flex',
						maxWidth: OG_WIDTH - OG_GRID_INSET * 2 - 36,
						color: colors.foreground,
						fontSize: titleFontSize,
						fontWeight: 500,
						letterSpacing: '-0.035em',
						lineHeight: titleLineHeight,
						textAlign: 'left',
						textWrapStyle: 'pretty'
					}
				},
				title
			),
			el(
				'div',
				{
					style: {
						display: 'flex',
						maxWidth: OG_WIDTH - OG_GRID_INSET - 300,
						color: withAlpha(colors.foreground, 0.58),
						fontSize: 25,
						fontWeight: 400,
						lineHeight: descriptionLineHeight,
						textAlign: 'left',
						textWrapStyle: 'pretty'
					}
				},
				description
			)
		)
	);

export async function createOgImage(options: OgImageOptions) {
	const response = new ImageResponse(createComponent(options), {
		width: OG_WIDTH,
		height: OG_HEIGHT,
		format: 'png',
		fonts: takumiFontLoaders,
		headers: {
			'content-type': 'image/png',
			'cache-control': 'public, max-age=3600'
		}
	});

	await response.ready;
	return response;
}
