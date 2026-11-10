import layoutCss from '../site/theme.css?raw';

type OgThemeColors = {
	background: string;
	foreground: string;
	foregroundMuted: string;
	accent: string;
	shadowHighlight: string;
};

export function extractCustomProperties(css: string, selector: string): Map<string, string> {
	const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const blockMatch = new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\}`).exec(css);

	if (!blockMatch) throw new Error(`Could not find the ${selector} theme block in site/theme.css`);

	const properties = new Map<string, string>();
	const propertyPattern = /(--[\w-]+)\s*:\s*([^;]+);/g;
	for (const match of blockMatch[1].matchAll(propertyPattern)) {
		properties.set(match[1], match[2].trim());
	}

	return properties;
}

export function resolveCustomProperty(
	name: string,
	properties: Map<string, string>,
	stack = new Set<string>()
): string {
	const value = properties.get(name);
	if (!value) throw new Error(`Could not find ${name} in the CSS theme tokens`);
	if (stack.has(name))
		throw new Error(`Circular CSS custom property reference detected for ${name}`);

	const nextStack = new Set(stack).add(name);
	return value.replace(/var\((--[\w-]+)\)/g, (_, referencedName: string) =>
		resolveCustomProperty(referencedName, properties, nextStack)
	);
}

export function toSvgColor(color: string): string {
	const match =
		/^oklch\(\s*([\d.]+)(%)?\s+([\d.]+)(%)?\s+([\d.]+)(?:deg)?(?:\s*\/\s*([\d.]+)(%)?)?\s*\)$/i.exec(
			color
		);

	if (!match) return color;

	const lightness = Number(match[1]) / (match[2] ? 100 : 1);
	const chroma = Number(match[3]) * (match[4] ? 0.004 : 1);
	const hue = (Number(match[5]) * Math.PI) / 180;
	const alpha = match[6] ? Number(match[6]) / (match[7] ? 100 : 1) : 1;
	const a = chroma * Math.cos(hue);
	const b = chroma * Math.sin(hue);

	const l = Math.pow(lightness + 0.3963377774 * a + 0.2158037573 * b, 3);
	const m = Math.pow(lightness - 0.1055613458 * a - 0.0638541728 * b, 3);
	const s = Math.pow(lightness - 0.0894841775 * a - 1.291485548 * b, 3);
	const linearRgb = [
		4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s
	];

	const toHexChannel = (channel: number) => {
		const clamped = Math.max(0, Math.min(1, channel));
		const srgb =
			clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
		return Math.round(srgb * 255)
			.toString(16)
			.padStart(2, '0');
	};

	const rgb = linearRgb.map(toHexChannel).join('');
	const alphaHex =
		alpha < 1
			? Math.round(Math.max(0, Math.min(1, alpha)) * 255)
					.toString(16)
					.padStart(2, '0')
			: '';

	return `#${rgb}${alphaHex}`;
}

export function withAlpha(color: string, alpha: number): string {
	const srgbColor = toSvgColor(color);
	const match = /^#([\da-f]{2})([\da-f]{2})([\da-f]{2})(?:[\da-f]{2})?$/i.exec(srgbColor);
	if (!match) return color;

	const [, red, green, blue] = match;
	const clampedAlpha = Math.max(0, Math.min(1, alpha));
	return `rgba(${Number.parseInt(red, 16).toString()}, ${Number.parseInt(green, 16).toString()}, ${Number.parseInt(blue, 16).toString()}, ${clampedAlpha.toString()})`;
}

const rootThemeProperties = extractCustomProperties(layoutCss, ':root');
const darkThemeProperties = new Map([
	...rootThemeProperties,
	...extractCustomProperties(layoutCss, '.dark')
]);

const resolveTheme = (properties: Map<string, string>): OgThemeColors => ({
	background: resolveCustomProperty('--background', properties),
	foreground: resolveCustomProperty('--foreground', properties),
	foregroundMuted: resolveCustomProperty('--muted-foreground', properties),
	accent: resolveCustomProperty('--primary', properties),
	shadowHighlight: resolveCustomProperty('--border', properties)
});

export const ogThemeColors = {
	light: resolveTheme(rootThemeProperties),
	dark: resolveTheme(darkThemeProperties)
} as const;
