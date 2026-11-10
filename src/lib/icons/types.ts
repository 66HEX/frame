import type { Component } from 'svelte';
import type { SVGAttributes } from 'svelte/elements';

export type IconProps = Omit<SVGAttributes<SVGSVGElement>, 'children'> & {
	size?: number | string;
	/** Retained for React and Vue framework logos; Nucleo UI glyphs keep their source stroke widths. */
	strokeWidth?: number | string;
	title?: string;
};

export type IconComponent = Component<IconProps>;
