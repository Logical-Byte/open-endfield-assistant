import { ref, type Ref, watch } from 'vue';

import type { SettingsDraft } from './model';

type UpdateSettingsDraft = Pick<SettingsDraft, 'mirrorchyanCdk' | 'updateProxyUrl'>;

export interface UpdateSettingsBuffers {
  mirrorchyanCdk: Ref<string>;
  updateProxyUrl: Ref<string>;
  commitMirrorchyanCdk(): void;
  commitUpdateProxyUrl(): void;
  handleMirrorchyanCdkKeydown(event: KeyboardEvent): void;
  handleUpdateProxyUrlKeydown(event: KeyboardEvent): void;
}

/**
 * 更新设置的文本控件缓冲。
 *
 * CDK 与代理地址只在用户结束输入时写入全局 draft，避免每个按键都触发保存或 CDK 加密。
 */
export function createUpdateSettingsBuffers(draft: UpdateSettingsDraft): UpdateSettingsBuffers {
  const mirrorchyanCdk = ref(draft.mirrorchyanCdk);
  const updateProxyUrl = ref(draft.updateProxyUrl);

  // 设置模块异步载入完成后，同步初始值到尚未编辑的输入框。
  watch(
    () => draft.mirrorchyanCdk,
    (value: string) => {
      mirrorchyanCdk.value = value;
    },
  );
  watch(
    () => draft.updateProxyUrl,
    (value: string) => {
      updateProxyUrl.value = value;
    },
  );

  function commitMirrorchyanCdk(): void {
    draft.mirrorchyanCdk = mirrorchyanCdk.value;
  }

  function commitUpdateProxyUrl(): void {
    draft.updateProxyUrl = updateProxyUrl.value;
  }

  function handleMirrorchyanCdkKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      commitMirrorchyanCdk();
    }
  }

  function handleUpdateProxyUrlKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      commitUpdateProxyUrl();
    }
  }

  return {
    mirrorchyanCdk,
    updateProxyUrl,
    commitMirrorchyanCdk,
    commitUpdateProxyUrl,
    handleMirrorchyanCdkKeydown,
    handleUpdateProxyUrlKeydown,
  };
}
