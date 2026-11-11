// The edge calculation and observers match portfolio's ScrollArea. This adapter
// applies them through viewportRef so the official shadcn component stays unchanged.
export function observeHorizontalOverflow(viewport: HTMLElement, label: string) {
	viewport.setAttribute('role', 'region');
	viewport.setAttribute('aria-label', label);

	const updateFade = () => {
		const maxScroll = Math.max(0, viewport.scrollWidth - viewport.clientWidth);
		const offset =
			getComputedStyle(viewport).direction === 'rtl'
				? maxScroll + viewport.scrollLeft
				: viewport.scrollLeft;
		// Clamp overscroll and shrink the fade smoothly when approaching either edge.
		const left = Math.max(0, Math.min(offset, maxScroll));
		const right = maxScroll - left;
		// Scroll dimensions are rounded, while scrollLeft can contain fractional pixels.
		const leftFade = left > 1 ? Math.min(24, left) : 0;
		const rightFade = right > 1 ? Math.min(24, right) : 0;
		viewport.style.maskImage =
			leftFade > 0 || rightFade > 0
				? `linear-gradient(to right, transparent, black ${leftFade.toString()}px, black calc(100% - ${rightFade.toString()}px), transparent)`
				: '';
	};

	const update = () => {
		const isScrollable = viewport.scrollWidth > viewport.clientWidth;
		viewport.toggleAttribute('data-scrollable', isScrollable);
		viewport.tabIndex = isScrollable ? 0 : -1;
		updateFade();
	};

	const observer = new ResizeObserver(update);
	observer.observe(viewport);
	if (viewport.firstElementChild) observer.observe(viewport.firstElementChild);
	viewport.addEventListener('scroll', updateFade, { passive: true });
	update();

	return () => {
		observer.disconnect();
		viewport.removeEventListener('scroll', updateFade);
	};
}
