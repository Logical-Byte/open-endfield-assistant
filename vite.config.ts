import ui from '@nuxt/ui/vite';
import vue from '@vitejs/plugin-vue';
import { readFileSync } from 'node:fs';
import process from 'node:process';
import { fileURLToPath, URL } from 'node:url';
import type { Plugin } from 'vite';
import { defineConfig } from 'vite';
import { createHtmlPlugin } from 'vite-plugin-html';
import vueRouter from 'vue-router/vite';

const host = process.env.TAURI_DEV_HOST;

// 从 resources 目录提供 favicon
function resourcesFavicon(): Plugin {
  const iconPath = fileURLToPath(new URL('./resources/icons/icon.ico', import.meta.url));
  const faviconUrl = '/favicon.ico';

  return {
    name: 'oea-resources-favicon',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url && req.url.split('?')[0] === faviconUrl) {
          res.setHeader('Content-Type', 'image/x-icon');
          res.end(readFileSync(iconPath));
          return;
        }
        next();
      });
    },
    generateBundle() {
      this.emitFile({
        type: 'asset',
        fileName: 'favicon.ico',
        source: readFileSync(iconPath),
      });
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    resourcesFavicon(),
    vueRouter({
      dts: 'src/route-map.d.ts',
    }),
    vue(),
    ui({
      ui: {
        colors: {
          primary: 'green',
          neutral: 'zinc',
        },
      },
    }),
    createHtmlPlugin({ minify: true }),
  ],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**', '**/EBWebView/**'],
    },
  },
});
