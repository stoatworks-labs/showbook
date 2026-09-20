import { defineConfig, type Plugin } from 'vitest/config';
import react from '@vitejs/plugin-react';

import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));

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
          html: html.replace('<html lang="en">', '<html lang="en" data-hosted>').replace('<title>Showbook</title>', '<title>Showbook lite</title>'),
          tags: [
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
