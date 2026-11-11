import { describe, expect, it } from 'vitest';
import { getDocsDocument, getNavigationGroups } from './docs';
import {
  contentSections,
  extractTocHeadings,
  getContentSectionHref,
  getContentSectionManifest,
  getContentSectionMetadata,
  getContentSectionModule,
  getContentSectionRawSource,
  getContentSectionTocHeadings
} from './sections';
import { getSearchIndex } from '../features/docs/docs-search-index';
import { searchDocsIndex } from '../features/docs/docs-search';
import { entries as pageEntries } from '../../routes/[section]/[...slug]/+page';
import { entries as rawEntries } from '../../routes/[section]/raw/[...slug]/+server';

const manifest = getContentSectionManifest('docs');
const pagesByHref = new Map(
  manifest.map((item) => [getContentSectionHref('docs', item.slug), item])
);

describe('Frame documentation migration', () => {
  it('publishes the docs view and prerenders every guide and its raw Markdown', () => {
    expect(contentSections.map((section) => section.id)).toEqual(['docs']);
    expect(pagesByHref.has('/docs')).toBe(true);
    expect(pagesByHref.has('/docs/installation')).toBe(true);
    expect(pagesByHref.has('/docs/getting-started')).toBe(true);

    const publishedPages = pageEntries();
    const publishedSources = rawEntries();
    for (const item of manifest) {
      const href = getContentSectionHref('docs', item.slug);
      const metadata = getContentSectionMetadata('docs', href);
      expect(metadata?.title, href).toBeTruthy();
      expect(metadata?.description, href).toBeTruthy();
      expect(getContentSectionRawSource('docs', item.slug), href).toBeTruthy();
      expect(publishedPages).toContainEqual({ section: 'docs', slug: item.slug });
      expect(publishedSources).toContainEqual({ section: 'docs', slug: item.slug || 'index' });
    }
  });

  it('resolves every internal guide link and section anchor in the real content', () => {
    for (const item of manifest) {
      const source = getContentSectionRawSource('docs', item.slug)!;
      for (const match of source.matchAll(/\]\((\/[^)\s]+)\)/g)) {
        const url = new URL(match[1], 'https://framegui.app');
        const target = pagesByHref.get(url.pathname);
        expect(target, `${item.slug || 'index'} links to ${match[1]}`).toBeDefined();
        if (url.hash && target) {
          const headings = getContentSectionTocHeadings('docs', target.slug, 'h1, h2, h3, h4, h5, h6');
          expect(headings.map((heading) => heading.id), match[1]).toContain(
            decodeURIComponent(url.hash.slice(1))
          );
        }
      }
    }
  });

  it('finds every visible guide by title and routes content matches to real headings', () => {
    const searchIndex = getSearchIndex('docs');
    const visiblePages = getNavigationGroups('docs').flatMap((group) => group.items);
    for (const page of visiblePages) {
      const document = getDocsDocument('docs', page.href)!;
      const results = searchDocsIndex(searchIndex, document.title);
      expect(results.map((group) => group.page.href), document.title).toContain(page.href);
    }

    const subtitles = searchDocsIndex(searchIndex, 'burn subtitles');
    expect(subtitles.flatMap((group) => group.children).map((result) => result.href)).toContain(
      '/docs/subtitles#burn-subtitles-into-the-picture'
    );
    expect(searchDocsIndex(searchIndex, 'FragCanvas')).toEqual([]);
  });

  it('does not resolve missing guides or the removed playground as a documentation page', async () => {
    expect(getContentSectionMetadata('docs', '/docs/not-a-real-guide')).toBeNull();
    expect(getContentSectionRawSource('docs', 'not-a-real-guide')).toBeNull();
    await expect(getContentSectionModule('docs', 'not-a-real-guide')).resolves.toBeNull();
    await expect(getContentSectionModule('playground', '')).resolves.toBeNull();
  });
});

describe('documentation heading text', () => {
  it('removes reconstructed tags without leaving tag fragments in labels', () => {
    const headings = extractTocHeadings(
      '## Read <scr<script>ipt>example</scr</script>ipt> safely',
      'h2'
    );
    expect(headings).toEqual([{ id: 'read-example-safely', text: 'Read example safely', level: 2 }]);
  });

  it('preserves adjacent formatted words and duplicate heading anchors', () => {
    const source = [
      '## **Video** <em>con</em>version and [audio](/docs/audio)',
      '## **Video** <em>con</em>version and [audio](/docs/audio)'
    ].join('\n');
    expect(extractTocHeadings(source, 'h2')).toEqual([
      { id: 'video-conversion-and-audio', text: 'Video conversion and audio', level: 2 },
      { id: 'video-conversion-and-audio-1', text: 'Video conversion and audio', level: 2 }
    ]);
  });

  it('preserves encoded angle brackets as plain text', () => {
    const [heading] = extractTocHeadings('## Literal &lt;script&gt; and &#60;video&#62;', 'h2');
    expect(heading.text).toBe('Literal <script> and <video>');
  });
});
