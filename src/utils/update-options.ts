import type { OeaConfig } from '@/types/oeaConfig';

export const UPDATE_SOURCE_ITEMS: Array<{
  label: string;
  value: OeaConfig['updateSource'];
}> = [
  { label: 'Mirror酱', value: 'mirrorchyan' },
  { label: 'GitHub', value: 'github' },
];

export const UPDATE_PROXY_MODE_ITEMS: Array<{
  label: string;
  value: OeaConfig['updateProxyMode'];
}> = [
  { label: '不使用代理', value: 'none' },
  { label: '系统代理', value: 'system' },
  { label: '自定义代理', value: 'custom' },
];
