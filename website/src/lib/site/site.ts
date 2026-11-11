import appleTouchIcon from './assets/apple-touch-icon.png';
import manifestIcon192 from './assets/web-app-manifest-192x192.png';
import manifestIcon512 from './assets/web-app-manifest-512x512.png';
import logoRaw from './assets/logo.svg?raw';

export const siteConfig = {
  name: 'Frame', logoRaw, language: 'en', locale: 'en_US',
  home: { href: '/', label: 'Frame home' },
  assets: { favicon: '/favicon.svg', appleTouchIcon, manifestIcons: [
    { src: manifestIcon192, sizes: '192x192', type: 'image/png', purpose: 'any' },
    { src: manifestIcon512, sizes: '512x512', type: 'image/png', purpose: 'any' }
  ] },
  shortName: 'Frame', url: 'https://framegui.app',
  description: 'Convert video, audio, and images in one native workspace. A free, open-source media converter powered by FFmpeg, with your files processed locally.',
  author: 'Marek Jóźwiak',
  keywords: ['Frame', 'media converter', 'FFmpeg GUI', 'video converter', 'audio converter', 'image converter', 'open source', 'macOS', 'Windows', 'Linux'],
  ogImage: '/og', themeColor: { light: '#e9e9eb', dark: '#111113' },
  links: { github: 'https://github.com/66HEX/frame', twitter: 'https://x.com/madebyhex' }
};
