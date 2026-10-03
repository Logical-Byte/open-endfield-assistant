import { ref } from 'vue';

/** 开发者选项的可见性在本次应用会话内保留，切换页面不会重置。 */
export const developerSettingsEnabled = ref<boolean>(false);
