const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';

export function prefersReducedMotion(): boolean {
	return typeof window !== 'undefined' && window.matchMedia(REDUCED_MOTION_QUERY).matches;
}
