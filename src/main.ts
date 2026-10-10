import App from '@/App.vue';
import { i18n } from '@/shared/i18n';
import I18NextVue from 'i18next-vue';
import '@/assets/css/main.css';
import ui from '@nuxt/ui/vue-plugin';
import { createHead } from '@unhead/vue/client';
import { createApp } from 'vue';
import { createRouter, createWebHistory } from 'vue-router';
import { handleHotUpdate, routes } from 'vue-router/auto-routes';

const app = createApp(App);

const head = createHead();
const router = createRouter({
  routes,
  history: createWebHistory(),
});

app.use(I18NextVue, { i18next: i18n });
app.use(head);
app.use(router);
app.use(ui);

app.mount('#app');

if (import.meta.hot) {
  handleHotUpdate(router);
}
