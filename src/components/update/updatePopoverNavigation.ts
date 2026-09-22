import type { Ref } from 'vue';

export interface UpdateSettingsRouter {
  push(to: string): Promise<unknown>;
}

/** 关闭更新弹窗，并打开设置页中唯一的更新设置编辑区域。 */
export function openUpdateSettings(popoverOpen: Ref<boolean>, router: UpdateSettingsRouter): void {
  popoverOpen.value = false;
  void router.push('/settings#update');
}
