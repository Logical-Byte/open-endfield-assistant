import { appVariant, AppVariant } from '@/app/build';

/** 当前应用形态允许的更新能力，不表示更新操作当前是否可执行。 */
export interface UpdateCapabilities {
  /** 是否允许检查、下载和安装在线更新包。 */
  readonly onlineUpdate: boolean;
  /** 是否允许通过开发者选项选择并安装本地更新包。 */
  readonly localInstall: boolean;
}

const capabilities: Record<AppVariant, UpdateCapabilities> = {
  [AppVariant.Dev]: { onlineUpdate: false, localInstall: false },
  [AppVariant.Release]: { onlineUpdate: true, localInstall: true },
};

export const updateCapabilities: UpdateCapabilities = capabilities[appVariant];
