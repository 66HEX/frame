<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { MediaQuery, SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { cn } from '$lib/utils';
	import { page } from '$app/state';
	import { ChevronDownIcon, TableOfContentsIcon } from '$lib/icons';
	import { Popover } from 'bits-ui';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import type { ContentTocHeading } from '$lib/content/sections';
	import { prefersReducedMotion } from '$lib/utils/motion';

	type TocItem = ContentTocHeading & {
		element: HTMLElement;
	};

	type IndicatorRange = {
		startId: string;
		endId: string;
	};

	type PathPoint = {
		x: number;
		y: number;
	};

	type Props = {
		selector?: string;
		title?: string;
		emptyLabel?: string;
		groupTitle: string;
		scrollContainerId?: string;
		headings?: ContentTocHeading[];
	};

	const props = $props();
	const selector = $derived(
		(props as Props).selector ?? '[data-doc-content] h2, [data-doc-content] h3'
	);
	const title = $derived((props as Props).title ?? 'On this page');
	const emptyLabel = $derived((props as Props).emptyLabel ?? 'No headings');
	const groupTitle = $derived((props as Props).groupTitle);
	const desktop = new MediaQuery('(min-width: 48rem)');
	let mobileOpen = $state(false);
	let selectedHeading: string | null = null;
	const scrollContainerId = $derived((props as Props).scrollContainerId ?? null);
	const initialHeadings = $derived(normalizeHeadings((props as Props).headings));

	let headings = $state<ContentTocHeading[]>([]);
	const renderedHeadings = $derived(headings.length > 0 ? headings : initialHeadings);
	let activeId = $state('');
	const activeHeading = $derived(renderedHeadings.find((heading) => heading.id === activeId));
	const progress = $derived(
		renderedHeadings.length > 0
			? ((renderedHeadings.findIndex((heading) => heading.id === activeId) + 1) /
					renderedHeadings.length) *
					100
			: 0
	);
	let indicatorTop = $state(0);
	let indicatorHeight = $state(0);
	let indicatorBottom = $state(0);
	let lineHeight = $state(0);
	let svgPath = $state('');
	let svgWidth = $state(40);
	let indicatorRange = $state<IndicatorRange | null>(null);
	let pendingIndicatorFrame: number | null = null;
	let lastPulsedRangeKey = '';
	let collectFrame: number | null = null;
	let collectCleanup: (() => void) | undefined;

	const ACTIVE_OFFSET = 140;
	const VISIBLE_BUFFER = 24;
	const CORNER_RADIUS = 2;
	const linkRefs = new SvelteMap<string, HTMLAnchorElement>();
	const linkPositions = new SvelteMap<string, { top: number; height: number }>();
	const headingOrder = new SvelteMap<string, number>();
	const pulseTimers = new SvelteMap<string, number>();
	const lastIndicatorIds = new SvelteSet<string>();
	let pulsingDotIds = $state<string[]>([]);
	const linksWrapperId = 'toc-links-wrapper';

	const currentPath = $derived(page.url.pathname);

	function getScrollContainer() {
		if (typeof document === 'undefined') return window;
		if (!scrollContainerId) return window;
		const element = document.getElementById(scrollContainerId);
		return element ?? window;
	}

	const slugify = (value: string) =>
		value
			.normalize('NFD')
			.replace(/[\u0300-\u036f]/g, '')
			.toLowerCase()
			.trim()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');

	function normalizeHeadings(value?: ContentTocHeading[]) {
		return Array.isArray(value)
			? value.filter(
					(heading): heading is ContentTocHeading =>
						typeof heading.id === 'string' &&
						heading.id.length > 0 &&
						typeof heading.text === 'string' &&
						heading.text.length > 0 &&
						typeof heading.level === 'number'
				)
			: [];
	}

	function syncHeadingOrder(nextHeadings: ContentTocHeading[]) {
		headingOrder.clear();
		nextHeadings.forEach(({ id }, index) => {
			headingOrder.set(id, index);
		});
	}

	function applyHeadings(nextHeadings: ContentTocHeading[]) {
		headings = nextHeadings;
		syncHeadingOrder(nextHeadings);
		activeId = nextHeadings[0]?.id ?? '';
		lineHeight = 0;
		indicatorTop = 0;
		indicatorHeight = 0;
		indicatorBottom = 0;
		indicatorRange = null;
		lastPulsedRangeKey = '';
		lastIndicatorIds.clear();
	}

	function resetTocState(options: { clearHeadings?: boolean } = {}) {
		if (options.clearHeadings ?? true) {
			headings = [];
			headingOrder.clear();
		}
		activeId = '';
		lineHeight = 0;
		indicatorTop = 0;
		indicatorHeight = 0;
		indicatorBottom = 0;
		indicatorRange = null;
		lastPulsedRangeKey = '';
		lastIndicatorIds.clear();
		pulsingDotIds = [];
		if (typeof window !== 'undefined') {
			pulseTimers.forEach((timer) => {
				window.clearTimeout(timer);
			});
		}
		pulseTimers.clear();
		if (typeof window !== 'undefined' && pendingIndicatorFrame !== null) {
			window.cancelAnimationFrame(pendingIndicatorFrame);
			pendingIndicatorFrame = null;
		}
	}

	function syncInitialHeadings() {
		applyHeadings(initialHeadings);
	}

	function pulseDot(id: string) {
		if (typeof window === 'undefined' || !id || prefersReducedMotion()) return;

		const existingTimer = pulseTimers.get(id);
		if (existingTimer) {
			window.clearTimeout(existingTimer);
		}

		pulsingDotIds = pulsingDotIds.filter((item) => item !== id);
		window.requestAnimationFrame(() => {
			pulsingDotIds = [...pulsingDotIds, id];
			const timer = window.setTimeout(() => {
				pulsingDotIds = pulsingDotIds.filter((item) => item !== id);
				pulseTimers.delete(id);
			}, 560);
			pulseTimers.set(id, timer);
		});
	}

	function pulseRange(range: IndicatorRange | null) {
		if (!range) return;
		const rangeKey = `${range.startId}:${range.endId}`;
		if (rangeKey === lastPulsedRangeKey) return;
		lastPulsedRangeKey = rangeKey;

		const startIndex = headingOrder.get(range.startId);
		const endIndex = headingOrder.get(range.endId);
		if (startIndex === undefined || endIndex === undefined) return;

		const min = Math.min(startIndex, endIndex);
		const max = Math.max(startIndex, endIndex);

		const nextIndicatorIds = headings
			.filter((_heading, index) => index >= min && index <= max)
			.map((heading) => heading.id);

		nextIndicatorIds.forEach((id) => {
			if (!lastIndicatorIds.has(id)) {
				pulseDot(id);
			}
		});

		lastIndicatorIds.clear();
		nextIndicatorIds.forEach((id) => {
			lastIndicatorIds.add(id);
		});
	}

	function clearCollectionWork() {
		if (typeof window !== 'undefined' && collectFrame !== null) {
			window.cancelAnimationFrame(collectFrame);
		}
		collectFrame = null;

		collectCleanup?.();
		collectCleanup = undefined;
	}

	function registerLink(node: HTMLElement, id?: string) {
		let currentId = id ?? '';

		const assign = () => {
			if (!currentId) return;
			linkRefs.set(currentId, node as HTMLAnchorElement);
		};

		assign();

		return {
			update(newId?: string) {
				if (newId === currentId) return;
				if (currentId) {
					linkRefs.delete(currentId);
					linkPositions.delete(currentId);
				}
				currentId = newId ?? '';
				assign();
			},
			destroy() {
				if (currentId) {
					linkRefs.delete(currentId);
					linkPositions.delete(currentId);
				}
			}
		};
	}

	function buildRoundedPath(points: PathPoint[], radius: number) {
		if (points.length === 0) return '';
		if (points.length === 1) {
			const [point] = points;
			return `M ${point.x.toString()} ${point.y.toString()}`;
		}

		const commands: string[] = [`M ${points[0].x.toString()} ${points[0].y.toString()}`];

		for (let i = 1; i < points.length; i++) {
			const point = points[i];
			const prev = points[i - 1];

			if (i === points.length - 1) {
				commands.push(` L ${point.x.toString()} ${point.y.toString()}`);
				continue;
			}

			const next = points[i + 1];
			const prevVecX = point.x - prev.x;
			const prevVecY = point.y - prev.y;
			const nextVecX = next.x - point.x;
			const nextVecY = next.y - point.y;
			const prevLen = Math.hypot(prevVecX, prevVecY);
			const nextLen = Math.hypot(nextVecX, nextVecY);

			if (prevLen === 0 || nextLen === 0) {
				commands.push(` L ${point.x.toString()} ${point.y.toString()}`);
				continue;
			}

			const prevDirX = prevVecX / prevLen;
			const prevDirY = prevVecY / prevLen;
			const nextDirX = nextVecX / nextLen;
			const nextDirY = nextVecY / nextLen;
			const dot = prevDirX * nextDirX + prevDirY * nextDirY;

			if (Math.abs(dot) > 0.999) {
				commands.push(` L ${point.x.toString()} ${point.y.toString()}`);
				continue;
			}

			const cornerRadius = Math.min(radius, prevLen / 2, nextLen / 2);
			const entryX = point.x - prevDirX * cornerRadius;
			const entryY = point.y - prevDirY * cornerRadius;
			const exitX = point.x + nextDirX * cornerRadius;
			const exitY = point.y + nextDirY * cornerRadius;

			commands.push(` L ${entryX.toString()} ${entryY.toString()}`);
			commands.push(
				` Q ${point.x.toString()} ${point.y.toString()} ${exitX.toString()} ${exitY.toString()}`
			);
		}

		return commands.join('');
	}

	function getLinksWrapperElement() {
		if (typeof document === 'undefined') return null;
		const node = document.getElementById(linksWrapperId);
		return node instanceof HTMLOListElement ? node : null;
	}

	function updateLayout() {
		const linksWrapper = getLinksWrapperElement();
		if (!linksWrapper || headings.length === 0) {
			lineHeight = 0;
			return;
		}

		linkPositions.clear();
		const polyline: PathPoint[] = [];
		let maxW = 0;
		const indentStep = 12;
		const strokeWidth = 1;
		const halfStroke = strokeWidth / 2;

		headings.forEach((heading) => {
			const node = linkRefs.get(heading.id);
			if (!node) return;

			const style = window.getComputedStyle(node);
			const paddingTop = parseFloat(style.paddingTop) || 0;
			const paddingBottom = parseFloat(style.paddingBottom) || 0;
			const positionTop = node.offsetTop + paddingTop;
			const positionBottom = node.offsetTop + node.offsetHeight - paddingBottom;
			const positionHeight = Math.max(0, positionBottom - positionTop);

			linkPositions.set(heading.id, {
				top: positionTop,
				height: positionHeight
			});

			const x = (heading.level - 2) * indentStep + halfStroke;
			const top = positionTop;
			const bottom = Math.max(positionTop, positionBottom);

			polyline.push({ x, y: top });
			polyline.push({ x, y: bottom });

			maxW = Math.max(maxW, x + halfStroke);
		});

		svgPath = buildRoundedPath(polyline, CORNER_RADIUS);
		svgWidth = Math.max(40, maxW + 10);
		lineHeight = linksWrapper.scrollHeight;
	}

	function updateIndicator(range?: IndicatorRange) {
		const appliedRange =
			range ?? indicatorRange ?? (activeId ? { startId: activeId, endId: activeId } : null);

		if (!appliedRange) {
			indicatorRange = null;
			indicatorTop = 0;
			indicatorHeight = 0;
			indicatorBottom = 0;
			return;
		}

		if (range) {
			indicatorRange = range;
		} else {
			indicatorRange ??= appliedRange;
		}

		const startPos = linkPositions.get(appliedRange.startId);
		const endPos = linkPositions.get(appliedRange.endId);

		if (!startPos || !endPos) {
			indicatorTop = 0;
			indicatorHeight = 0;
			indicatorBottom = 0;
			return;
		}

		const top = Math.min(startPos.top, endPos.top);
		const bottom = Math.max(startPos.top + startPos.height, endPos.top + endPos.height);

		indicatorTop = top;
		indicatorHeight = Math.max(0, bottom - top);
		indicatorBottom = bottom;
	}

	function scheduleIndicatorUpdate(range?: IndicatorRange | null) {
		if (typeof window === 'undefined') {
			if (range) {
				pulseRange(range);
				updateIndicator(range);
			} else {
				updateIndicator();
			}
			return;
		}

		if (pendingIndicatorFrame !== null) {
			window.cancelAnimationFrame(pendingIndicatorFrame);
		}

		pendingIndicatorFrame = window.requestAnimationFrame(() => {
			pendingIndicatorFrame = null;
			if (range) {
				pulseRange(range);
				updateIndicator(range);
			} else {
				updateIndicator();
			}
		});
	}

	function collectDomHeadings(): TocItem[] {
		const slugCounts = new SvelteMap<string, number>();
		const usedIds = new SvelteSet<string>();
		const nodeList = Array.from(document.querySelectorAll(selector)).filter(
			(node): node is HTMLElement => node instanceof HTMLElement
		);

		const parsed: TocItem[] = [];

		for (const node of nodeList) {
			const rawText = node.textContent;
			const text = rawText ? rawText.trim() : '';
			if (!text) continue;

			let id = node.id;
			if (!id) {
				let baseSlug = slugify(text);
				if (!baseSlug) {
					baseSlug = `section-${(parsed.length + 1).toString()}`;
				}
				const count = slugCounts.get(baseSlug);
				if (typeof count === 'number') {
					const nextCount = count + 1;
					slugCounts.set(baseSlug, nextCount);
					baseSlug = `${baseSlug}-${nextCount.toString()}`;
				} else {
					slugCounts.set(baseSlug, 0);
				}
				id = baseSlug;
			}

			if (usedIds.has(id)) {
				const baseId = id;
				let nextCount = slugCounts.get(baseId) ?? 0;

				do {
					nextCount += 1;
					id = `${baseId}-${nextCount.toString()}`;
				} while (usedIds.has(id));

				slugCounts.set(baseId, nextCount);
			}

			if (node.id !== id) {
				node.id = id;
			}
			usedIds.add(id);

			const level = Number(node.tagName.replace('H', '')) || 2;
			parsed.push({
				id,
				text,
				level,
				element: node
			});
		}

		return parsed;
	}

	function resolveInitialHeadingElements(): TocItem[] {
		if (initialHeadings.length === 0) return [];

		const parsed = initialHeadings.flatMap((heading): TocItem[] => {
			const element = document.getElementById(heading.id);
			return element instanceof HTMLElement ? [{ ...heading, element }] : [];
		});

		return parsed.length === initialHeadings.length ? parsed : [];
	}

	function collectHeadings() {
		if (typeof document === 'undefined') {
			resetTocState({ clearHeadings: false });
			return undefined;
		}

		const initialParsed = resolveInitialHeadingElements();
		const parsed = initialParsed.length > 0 ? initialParsed : collectDomHeadings();

		applyHeadings(parsed.map(({ element: _element, ...rest }) => rest));

		lineHeight = 0;
		indicatorTop = 0;
		indicatorHeight = 0;
		indicatorBottom = 0;
		indicatorRange = null;

		requestAnimationFrame(() => {
			updateLayout();
		});

		if (!parsed.length) {
			return undefined;
		}

		const updateActive = () => {
			let current = parsed[0]?.id ?? '';
			const scrollEl = getScrollContainer();
			const isWindow = scrollEl === window;
			const scrollY = isWindow ? window.scrollY : (scrollEl as HTMLElement).scrollTop;
			const viewportHeight = isWindow ? window.innerHeight : (scrollEl as HTMLElement).clientHeight;
			const scrollHeight = isWindow
				? document.documentElement.scrollHeight
				: (scrollEl as HTMLElement).scrollHeight;
			const containerBounds = isWindow
				? { top: 0, bottom: viewportHeight }
				: (scrollEl as HTMLElement).getBoundingClientRect();
			const viewportTop = containerBounds.top - VISIBLE_BUFFER;
			const viewportBottom = containerBounds.bottom + VISIBLE_BUFFER;
			const visibleIds: string[] = [];

			for (const item of parsed) {
				const rect = item.element.getBoundingClientRect();
				if (rect.bottom >= viewportTop && rect.top <= viewportBottom) {
					visibleIds.push(item.id);
				}
				if (rect.top - ACTIVE_OFFSET <= viewportTop + VISIBLE_BUFFER) {
					current = item.id;
				}
			}

			const last = parsed[parsed.length - 1];
			const scrolledBottom = scrollY + viewportHeight;
			if (scrolledBottom >= scrollHeight - 20) {
				current = last.id;
			}

			activeId = current;
			const range: IndicatorRange | null =
				visibleIds.length > 0
					? {
							startId: visibleIds[0],
							endId: visibleIds[visibleIds.length - 1]
						}
					: current
						? { startId: current, endId: current }
						: null;

			scheduleIndicatorUpdate(range);
		};

		const container = getScrollContainer();

		if (parsed.length > 0) {
			updateActive();
		}

		const handleResize = () => {
			updateActive();
			updateLayout();
		};

		container.addEventListener('scroll', updateActive, { passive: true });
		window.addEventListener('resize', handleResize);

		return () => {
			container.removeEventListener('scroll', updateActive);
			window.removeEventListener('resize', handleResize);
			pulseTimers.forEach((timer) => {
				window.clearTimeout(timer);
			});
			pulseTimers.clear();
			pulsingDotIds = [];
			lastIndicatorIds.clear();
			if (pendingIndicatorFrame !== null) {
				window.cancelAnimationFrame(pendingIndicatorFrame);
				pendingIndicatorFrame = null;
			}
		};
	}

	function isLinkHighlighted(id: string) {
		if (!indicatorRange) {
			return activeId === id;
		}

		const startIndex = headingOrder.get(indicatorRange.startId);
		const endIndex = headingOrder.get(indicatorRange.endId);
		const currentIndex = headingOrder.get(id);

		if (startIndex === undefined || endIndex === undefined || currentIndex === undefined) {
			return activeId === id;
		}

		const min = Math.min(startIndex, endIndex);
		const max = Math.max(startIndex, endIndex);

		return currentIndex >= min && currentIndex <= max;
	}

	function manageToc(
		_node: HTMLElement,
		deps: { path: string; desktop: boolean; selector: string }
	) {
		let currentDeps = deps;
		const run = () => {
			clearCollectionWork();
			syncInitialHeadings();
			collectFrame = window.requestAnimationFrame(() => {
				collectFrame = null;
				collectCleanup = collectHeadings();
			});
		};
		run();
		return {
			update(nextDeps: typeof deps) {
				if (
					nextDeps.path === currentDeps.path &&
					nextDeps.desktop === currentDeps.desktop &&
					nextDeps.selector === currentDeps.selector
				)
					return;
				currentDeps = nextDeps;
				mobileOpen = false;
				selectedHeading = null;
				run();
			},
			destroy() {
				clearCollectionWork();
				resetTocState();
			}
		};
	}

	function observeLinksWrapper(node: HTMLOListElement) {
		const observer = new ResizeObserver(() => {
			updateLayout();
			updateIndicator();
		});
		observer.observe(node);
		return { destroy: () => observer.disconnect() };
	}

	function selectHeading(event: MouseEvent, id: string) {
		if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
		pulseDot(id);
		if (!desktop.current) {
			selectedHeading = id;
			mobileOpen = false;
		}
	}

	function restoreFocus(event: Event) {
		if (!selectedHeading) return;
		event.preventDefault();
		const heading = document.getElementById(selectedHeading);
		if (heading) {
			heading.tabIndex = -1;
			heading.focus({ preventScroll: true });
		}
		selectedHeading = null;
	}
</script>

<div
	class="contents"
	{@attach fromAction(manageToc, () => ({
		path: currentPath,
		desktop: desktop.current,
		selector
	}))}
>
	{#if desktop.current}
		{@render tocList()}
	{:else}
		<Popover.Root bind:open={mobileOpen}>
			<Popover.Trigger
				data-mobile-toc-trigger
				aria-label={`${title}: ${mobileOpen ? groupTitle : (activeHeading?.text ?? groupTitle)}`}
				class="flex min-h-11 w-full items-center gap-2.5 px-5 py-2.5 text-start text-sm text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-inset"
			>
				<!-- A data visualization of heading progress, rather than an interface icon. -->
				<span
					role="progressbar"
					aria-label={title}
					aria-valuemin={0}
					aria-valuemax={100}
					aria-valuenow={Math.round(progress)}
					class="toc-progress size-[18px] shrink-0 rounded-full"
					style={`--toc-progress: ${progress.toString()}%`}
				></span>
				<span class="min-w-0 flex-1 truncate" class:text-foreground={mobileOpen}>
					{mobileOpen ? groupTitle : (activeHeading?.text ?? groupTitle)}
				</span>
				<ChevronDownIcon
					size={18}
					class={cn(
						'shrink-0 transition-transform duration-150 motion-reduce:transition-none',
						mobileOpen && 'rotate-180'
					)}
				/>
			</Popover.Trigger>
			<!-- Keep height auto: fit-content collapses this panel's contents in Safari 26.0. -->
			<Popover.ContentStatic
				role="dialog"
				aria-label={title}
				trapFocus={false}
				onOpenAutoFocus={(event) => event.preventDefault()}
				onCloseAutoFocus={restoreFocus}
				class="absolute inset-x-0 top-full flex max-h-[60dvh] flex-col overflow-hidden border-y border-border bg-background/95 px-6 py-3 backdrop-blur-md outline-none"
			>
				{@render tocList()}
			</Popover.ContentStatic>
		</Popover.Root>
	{/if}
</div>

{#snippet tocList()}
	{#if renderedHeadings.length > 0}
		<nav class="flex h-full min-h-0 flex-col" aria-label={title}>
			{#if desktop.current}<div
					class="flex flex-none items-center gap-2 text-xs font-medium tracking-wide text-muted-foreground/70 uppercase"
				>
					<TableOfContentsIcon size={16} />
					{title}
				</div>{/if}
			<!-- Let the dot glow extend into the panel gutter without moving the links. -->
			<ScrollArea
				class="-ml-6 min-h-0 flex-1"
				viewportClass="max-h-[calc(60dvh-1.5rem-1px)] overscroll-contain py-2 pl-6 md:max-h-none"
				viewportStyle="mask-image: linear-gradient(to bottom, transparent, black 8px, black calc(100% - 8px), transparent); -webkit-mask-image: linear-gradient(to bottom, transparent, black 8px, black calc(100% - 8px), transparent);"
			>
				<div class="relative mx-1 flex">
					<div
						class="pointer-events-none absolute top-0 left-[2.5px] h-full w-10"
						style={`
	                    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 ${svgWidth.toString()} ${lineHeight.toString()}' width='${svgWidth.toString()}' height='${lineHeight.toString()}' preserveAspectRatio='none'%3E%3Cpath d='${svgPath}' stroke='black' stroke-width='1' fill='none'/%3E%3C/svg%3E");
                    -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 ${svgWidth.toString()} ${lineHeight.toString()}' width='${svgWidth.toString()}' height='${lineHeight.toString()}' preserveAspectRatio='none'%3E%3Cpath d='${svgPath}' stroke='black' stroke-width='1' fill='none'/%3E%3C/svg%3E");
                    mask-repeat: no-repeat;
                    -webkit-mask-repeat: no-repeat;
                    mask-position: left top;
                    -webkit-mask-position: left top;
                    mask-size: 100% 100%;
                    -webkit-mask-size: 100% 100%;
                `}
					>
						<div class="absolute inset-0 h-full w-full bg-border"></div>

						{#if indicatorHeight > 0}
							<div
								class="toc-active-line absolute left-0 w-full transition-[top,bottom] duration-450 ease-out motion-reduce:transition-none"
								style={`
	                            top: ${indicatorTop.toString()}px;
	                            bottom: ${Math.max(0, lineHeight - indicatorBottom).toString()}px;
                        `}
							></div>
						{/if}
					</div>

					<ol
						id={linksWrapperId}
						class="relative flex flex-col text-sm"
						{@attach fromAction(observeLinksWrapper)}
					>
						{#each renderedHeadings as heading (heading.id)}
							<li
								class="transition-colors duration-150 ease-out motion-reduce:transition-none"
								style={`padding-left: ${((heading.level - 2) * 12).toString()}px`}
							>
								<a
									href={`#${heading.id}`}
									onclick={(event) => selectHeading(event, heading.id)}
									aria-current={activeId === heading.id ? 'location' : undefined}
									class={cn(
										'flex min-h-8 items-center gap-2 rounded-xs py-1 font-medium tracking-normal transition-[color,box-shadow] duration-150 ease-out outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-inset motion-reduce:transition-none md:max-w-48',
										isLinkHighlighted(heading.id)
											? 'text-primary'
											: 'text-muted-foreground hover:text-foreground'
									)}
									{@attach fromAction(registerLink, () => heading.id)}
								>
									<span
										aria-hidden="true"
										class={cn(
											'toc-dot relative size-1.5 flex-none rounded-full transition-[background-color,box-shadow,scale] duration-150 ease-out motion-reduce:transform-none motion-reduce:transition-none',
											isLinkHighlighted(heading.id) && 'toc-dot-active',
											pulsingDotIds.includes(heading.id) && 'toc-dot-pulse'
										)}
									></span>
									<span
										class={cn(
											'min-w-0 truncate pl-1',
											isLinkHighlighted(heading.id) && 'text-primary'
										)}
									>
										{heading.text}
									</span>
								</a>
							</li>
						{/each}
					</ol>
				</div>
			</ScrollArea>
		</nav>
	{:else}
		<div class="text-sm text-muted-foreground">{emptyLabel}</div>
	{/if}
{/snippet}

<style>
	.toc-progress {
		background: conic-gradient(
			currentColor var(--toc-progress),
			color-mix(in oklch, currentColor 25%, transparent) 0
		);
		mask-image: radial-gradient(farthest-side, transparent calc(100% - 1.5px), black 0);
	}

	.toc-active-line {
		background-image: linear-gradient(
			to bottom,
			transparent,
			oklch(from var(--primary) l c h / 0.68) 22%,
			var(--primary) 50%,
			oklch(from var(--primary) l c h / 0.68) 78%,
			transparent
		);
		filter: drop-shadow(0 0 6px oklch(from var(--primary) l c h / 0.38));
	}

	.toc-dot {
		background-color: var(--muted-foreground);
		box-shadow: 0 0 0 2px var(--background);
		opacity: 0.72;
	}

	.toc-dot-active {
		background-color: var(--primary);
		box-shadow:
			inset 0 1px oklch(from var(--shadow-light) l c h / 0.35),
			0 0 0 2px var(--background),
			0 0 10px oklch(from var(--primary) l c h / 0.38);
		opacity: 1;
	}

	.toc-dot-active::after {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: 9999px;
		box-shadow: 0 0 9px oklch(from var(--primary) l c h / 0.5);
	}

	.toc-dot-pulse {
		animation: toc-dot-pulse 0.52s ease-out both;
	}

	@keyframes toc-dot-pulse {
		0% {
			transform: scale(1);
			box-shadow: 0 0 0 2px var(--background);
		}

		12% {
			transform: scale(1.15);
			background-color: var(--primary);
			box-shadow:
				0 0 0 2px var(--background),
				0 0 0 3px oklch(from var(--primary) l c h / 0.18),
				0 0 18px oklch(from var(--primary) l c h / 0.52);
		}

		100% {
			transform: scale(1);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.toc-dot-pulse {
			animation: none;
		}
	}
</style>
