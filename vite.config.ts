import ui from '@nuxt/ui/vite';
import vue from '@vitejs/plugin-vue';
import { readFileSync } from 'node:fs';
import process from 'node:process';
import { fileURLToPath, URL } from 'node:url';
import { valid } from 'semver';
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

/**
 * 从 `src-tauri/tauri.conf.json` 读取应用版本，并用 semver 严格校验
 * （编译期注入，与打包产物版本保持一致）。
 */
function readOeaVersion(): string {
  const config = JSON.parse(
    readFileSync(fileURLToPath(new URL('./src-tauri/tauri.conf.json', import.meta.url)), 'utf-8'),
  ) as { version?: string };
  if (!config.version) {
    throw new Error('`src-tauri/tauri.conf.json` 缺少 `version` 字段');
  }
  if (valid(config.version) !== config.version) {
    throw new Error(
      `\`src-tauri/tauri.conf.json\` 的 \`version\` 不是干净的 semver（如 \`0.1.0\`，不接受 \`v\` 前缀或构建元数据）: ${config.version}`,
    );
  }
  return config.version;
}

// https://vitejs.dev/config/
export default defineConfig(() => {
  const oeaVersion = readOeaVersion();

  return {
    plugins: [
      resourcesFavicon(),
      vueRouter({
        dts: 'src/route-map.d.ts',
      }),
      vue(),
      ui({
        ui: {
          colors: {
            primary: 'pink',
            neutral: 'zinc',
          },
        },
      }),
      createHtmlPlugin({ minify: true }),
    ],

    define: {
      __OEA_VERSION__: JSON.stringify(oeaVersion),
    },

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
  };
});
