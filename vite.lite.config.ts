import { defineConfig, type Plugin } from 'vitest/config';
import type { HtmlTagDescriptor } from 'vite';
import react from '@vitejs/plugin-react';

import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));

/**
 * What a search engine, a link preview or an LLM fetcher sees of showbook-lite:
 * the title and description, a canonical, the share card, JSON-LD with the same
 * @id as the entry on stoatworks-labs.com/web-tools/showbook-lite/ (so the two
 * read as one thing described twice), and a <noscript> summary for a reader
 * that doesn't run the app. Injected here, not in index.html, for the same
 * reason as everything else in hosted(): that file is the desktop app's too.
 */
const SITE = 'https://showbook-lite.stoatworks-labs.com';
const TITLE = "Showbook lite — Your switchers' show files, in a browser tab: inspect, document, convert";
const DESCRIPTION =
  'Barco Event Master and Analog Way LivePremier show files in a browser tab: inspect, edit, convert between them, and make PDFs, test patterns and Companion pages. Free.';
const SUMMARY = [
  'The browser build of Showbook, a show file library for video switchers: Barco Event Master (E2, S3-4K, EX, Encore3) and Analog Way LivePremier (Aquilon). It does everything that doesn\'t need a device: import, inspect, edit, PDF documentation, test patterns, conversion from one platform to the other, and Companion pages.',
  'It uses the same Rust parsers as the desktop app, compiled to WebAssembly, and keeps the library in the browser\'s own storage, so the show files you open stay on your machine. Pulling from and pushing to the live hardware needs the desktop app, which is free.',
];

function seoTags(): HtmlTagDescriptor[] {
  const meta = (attrs: Record<string, string>): HtmlTagDescriptor => ({ tag: 'meta', injectTo: 'head', attrs });
  const ld = {
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    '@id': `${SITE}#app`,
    name: 'Showbook lite',
    url: SITE,
    description: DESCRIPTION,
    applicationCategory: 'MultimediaApplication',
    operatingSystem: 'Any (runs in a web browser)',
    isAccessibleForFree: true,
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'GBP' },
    author: { '@id': 'https://stoatworks-labs.com/#organization', '@type': 'Organization', name: 'Stoatworks Labs', url: 'https://stoatworks-labs.com/' },
    publisher: { '@id': 'https://stoatworks-labs.com/#organization' },
    sameAs: ['https://stoatworks-labs.com/web-tools/showbook-lite/', 'https://github.com/stoatworks-labs/showbook'],
  };
  const links: [string, string][] = [
    ['https://stoatworks-labs.com/web-tools/showbook-lite/', 'Showbook lite on Stoatworks Labs'],
    ['https://stoatworks-labs.com/software/showbook/', 'Project page, with the desktop app'],
    ['https://stoatworks-labs.com/software/showbook/guide/', 'User guide'],
    ['https://github.com/stoatworks-labs/showbook', 'Source code on GitHub'],
  ];
  const noscript = [
    '<h1>Showbook lite</h1>',
    ...SUMMARY.map((p) => `<p>${p}</p>`),
    '<p>It needs JavaScript to run.</p>',
    `<ul>${links.map(([href, text]) => `<li><a href="${href}">${text}</a></li>`).join('')}</ul>`,
  ].join('');
  return [
    meta({ name: 'description', content: DESCRIPTION }),
    { tag: 'link', injectTo: 'head', attrs: { rel: 'canonical', href: `${SITE}/` } },
    meta({ property: 'og:type', content: 'website' }),
    meta({ property: 'og:site_name', content: 'Stoatworks Labs' }),
    meta({ property: 'og:title', content: TITLE }),
    meta({ property: 'og:description', content: DESCRIPTION }),
    meta({ property: 'og:url', content: `${SITE}/` }),
    meta({ property: 'og:image', content: 'https://stoatworks-labs.com/thumbs/showbook.png' }),
    meta({ property: 'og:image:width', content: '1280' }),
    meta({ property: 'og:image:height', content: '720' }),
    meta({ property: 'og:image:alt', content: 'Showbook lite' }),
    meta({ name: 'twitter:card', content: 'summary_large_image' }),
    { tag: 'script', injectTo: 'head', attrs: { type: 'application/ld+json' }, children: JSON.stringify(ld, null, 2) },
    { tag: 'noscript', injectTo: 'body-prepend', children: noscript },
  ];
}

/**
 * showbook-lite: the same front end, built for a browser at
 * showbook-lite.stoatworks-labs.com, with the Rust core as WebAssembly
 * (lite/pkg) and the library in IndexedDB. `__LITE__` is what switches the
 * API layer (src/lib/ipc.ts); nothing else about the source knows which build
 * it is in beyond a few `isLite` checks for what a browser cannot do.
 *
 * The support footer is appended here and not in index.html, because that
 * file is the desktop app's entry as well: a tag there would ship the funding
 * footer inside the window of someone who has already installed it. This
 * config is the hosted target and nothing else builds through it.
 *
 * publicDir is lite/public (the footer, _headers); the About dialog's two
 * files and the demo captures are copied from public/ at build time so there
 * is one copy of each — sync-about.py writes public/, not here.
 */
function hosted(): Plugin {
  const shared = ['about.js', 'about-data.js', ...readdirSync(resolve(import.meta.dirname, 'public/demo')).map((f) => `demo/${f}`)];
  return {
    name: 'showbook-lite-hosted',
    transformIndexHtml: {
      order: 'post',
      handler(html) {
        return {
          html: html.replace('<html lang="en">', '<html lang="en" data-hosted>').replace('<title>Showbook</title>', `<title>${TITLE}</title>`),
          tags: [
            ...seoTags(),
            // Installable-app head tags. They go here rather than in
            // index.html for the same reason the support footer does: that
            // file is the desktop app's entry too, and a manifest link in the
            // Tauri window would be pointing at a file that build never emits.
            { tag: 'link', injectTo: 'head', attrs: { rel: 'manifest', href: '/manifest.webmanifest' } },
            // iOS ignores the manifest's icons entirely and wants a PNG of its own.
            { tag: 'link', injectTo: 'head', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
            // Tints the status bar and the title bar once installed.
            { tag: 'meta', injectTo: 'head', attrs: { name: 'theme-color', content: '#0b0d12' } },
            {
              tag: 'script',
              injectTo: 'body',
              attrs: {
                src: '/support-footer.js',
                defer: true,
                'data-app': 'Showbook lite',
                'data-repo': 'https://github.com/stoatworks-labs/showbook',
                'data-version': `v${pkg.version}`,
                'data-note': 'Everything runs in your browser: the show files you open stay on your machine, in this browser\'s own storage.',
              },
            },
          ],
        };
      },
    },
    generateBundle() {
      for (const f of shared) {
        this.emitFile({ type: 'asset', fileName: f, source: readFileSync(resolve(import.meta.dirname, 'public', f)) });
      }
    },
    configureServer(server) {
      // The dev server serves publicDir only; the shared files come from public/.
      server.middlewares.use((req, res, next) => {
        const f = shared.find((s) => req.url === `/${s}`);
        if (!f) return next();
        res.setHeader('Content-Type', f.endsWith('.json') ? 'application/json' : 'text/javascript');
        res.end(readFileSync(resolve(import.meta.dirname, 'public', f)));
      });
    },
  };
}

export default defineConfig({
  define: { __APP_VERSION__: JSON.stringify(`v${pkg.version}`), __LITE__: 'true' },
  plugins: [react(), hosted()],
  resolve: { alias: { 'showbook-core': resolve(import.meta.dirname, 'lite/pkg/showbook.js') } },
  publicDir: resolve(import.meta.dirname, 'lite/public'),
  base: '/',
  clearScreen: false,
  server: {
    port: 5179,
    strictPort: true,
  },
  build: {
    outDir: 'dist-lite',
    sourcemap: true,
    target: 'es2023',
  },
});
