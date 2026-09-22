import { describe, expect, it } from 'vitest';

import { createUpdateSettingsBuffers } from './updateSettingsEditor';

describe('更新设置文本输入缓冲', () => {
  it('仅在 CDK 输入框失焦或按 Enter 后提交 draft', () => {
    const draft = { mirrorchyanCdk: '已保存 CDK', updateProxyUrl: '' };
    const buffers = createUpdateSettingsBuffers(draft);

    buffers.mirrorchyanCdk.value = '正在输入的新 CDK';
    expect(draft.mirrorchyanCdk).toBe('已保存 CDK');

    buffers.handleMirrorchyanCdkKeydown({ key: 'Escape' } as KeyboardEvent);
    expect(draft.mirrorchyanCdk).toBe('已保存 CDK');

    buffers.handleMirrorchyanCdkKeydown({ key: 'Enter' } as KeyboardEvent);
    expect(draft.mirrorchyanCdk).toBe('正在输入的新 CDK');

    buffers.mirrorchyanCdk.value = '失焦后提交的 CDK';
    buffers.commitMirrorchyanCdk();
    expect(draft.mirrorchyanCdk).toBe('失焦后提交的 CDK');
  });

  it('仅在代理地址输入框失焦或按 Enter 后提交 draft', () => {
    const draft = { mirrorchyanCdk: '', updateProxyUrl: 'http://127.0.0.1:7890' };
    const buffers = createUpdateSettingsBuffers(draft);

    buffers.updateProxyUrl.value = 'http://127.0.0.1:1080';
    expect(draft.updateProxyUrl).toBe('http://127.0.0.1:7890');

    buffers.handleUpdateProxyUrlKeydown({ key: 'Enter' } as KeyboardEvent);
    expect(draft.updateProxyUrl).toBe('http://127.0.0.1:1080');

    buffers.updateProxyUrl.value = 'http://127.0.0.1:1081';
    buffers.commitUpdateProxyUrl();
    expect(draft.updateProxyUrl).toBe('http://127.0.0.1:1081');
  });
});
