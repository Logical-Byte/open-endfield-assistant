import type zhCn from './locales/zh-CN.json';
type MessageSchema = typeof zhCn;
declare module 'vue-i18n' {
  // 声明合并需要 interface，字典 schema 已拥有完整成员。
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  export interface DefineLocaleMessage extends MessageSchema {}
}
