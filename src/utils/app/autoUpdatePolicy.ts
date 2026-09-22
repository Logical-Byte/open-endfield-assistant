import type { SettingsSnapshot } from '@/modules/settings/model';

/** 自动更新编排只需要的已生效设置。 */
export type AutoUpdateSettings = Pick<
  SettingsSnapshot,
  'autoDownloadUpdates' | 'autoInstallUpdates'
>;

export type AutomaticUpdateAction = 'download' | 'install' | 'none';

/**
 * 根据已生效设置决定更新编排的下一步自动动作。
 *
 * draft 仅代表尚未保存的编辑，不能参与这里的决策。
 */
export function decideAutomaticUpdateAction(
  settings: Readonly<AutoUpdateSettings>,
  update: 'available' | 'pending',
): AutomaticUpdateAction {
  if (update === 'available') {
    return settings.autoDownloadUpdates ? 'download' : 'none';
  }
  return settings.autoInstallUpdates ? 'install' : 'none';
}
