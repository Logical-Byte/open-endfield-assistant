import App from '@/App.vue';
import '@/assets/css/main.css';
import ui from '@nuxt/ui/vue-plugin';
import { createHead } from '@unhead/vue/client';
import { createApp } from 'vue';
import { createRouter, createWebHistory } from 'vue-router';
import { handleHotUpdate, routes } from 'vue-router/auto-routes';

// 重导出常量，以便在 Vue 模板中使用
/** 应用版本号（编译期注入，来自 `src-tauri/tauri.conf.json`）。 */
export const oeaVersion = __OEA_VERSION__;

export const app = createApp(App);

const head = createHead();
const router = createRouter({
  routes,
  history: createWebHistory(),
});

app.use(head);
app.use(router);
app.use(ui);

app.mount('#app');

if (import.meta.hot) {
  handleHotUpdate(router);
}
