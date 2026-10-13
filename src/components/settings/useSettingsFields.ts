import { toValue, type MaybeRefOrGetter } from 'vue';
import type {
  ChoiceButtonsSettingsItem,
  InputSettingsItem,
  InputOptions,
  SelectSettingsItem,
  SettingsChoice,
  SettingsItemOptions,
  SliderSettingsItem,
  SliderOptions,
  SwitchSettingsItem,
} from './settingsItem';

/** 从数据源中选择值类型匹配的字段，避免把数值字段连接到开关等控件。 */
type FieldKey<T, Value> = {
  [Key in keyof T]-?: T[Key] extends Value ? Key : never;
}[keyof T];

/** 字段工厂的具名接口，返回具体设置项类型，便于 LSP 显示和跳转。 */
export interface SettingsFields<T extends object> {
  /** 布尔字段。options 可覆盖工厂的默认展示状态。 */
  switch(key: FieldKey<T, boolean>, options?: SettingsItemOptions): SwitchSettingsItem;
  /** 数值字段。声明滑块范围、步长、格式以及可选展示状态。 */
  slider(key: FieldKey<T, number>, options: SliderOptions): SliderSettingsItem;
  /** 字符串字段。选项值根据字段类型约束，文案可通过响应式 getter 更新。 */
  select<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
    options?: SettingsItemOptions,
  ): SelectSettingsItem;
  /** 文本或可空文本字段。保留 lazy 输入和 null 的业务含义。 */
  input(key: FieldKey<T, string | null>, options?: InputOptions): InputSettingsItem;
  /** 字符串字段的按钮组展示，例如语言选择。 */
  choices<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
    options?: SettingsItemOptions,
  ): ChoiceButtonsSettingsItem;
}

/**
 * 将只读数据投影和 patch 编辑入口连接为声明式设置项。
 *
 * source 接受 ref、getter 或对象。每次 read 都访问最新投影，描述中不缓存字段值。
 * edit 负责业务规范化、保存及生效时机，本 composable 不依赖任何业务 store。
 * defaults 为一组字段指定共同的展示状态，逐项 options 可以覆盖。state 保留 ref/getter，
 * 不在创建时求值。默认值和逐项选项在创建设置项时组合，其后响应式更新由 ref/getter 提供。
 *
 * @example
 * const fields: SettingsFields<DraftSettings> = useSettingsFields(draft, edit, { state });
 * const trayItem: SwitchSettingsItem = fields.switch('minimizeToTray');
 */
export function useSettingsFields<T extends object>(
  source: MaybeRefOrGetter<Readonly<T> | null>,
  edit: (patch: Partial<T>) => void,
  defaults: SettingsItemOptions = {},
): SettingsFields<T> {
  function read<Key extends keyof T>(key: Key): T[Key] | undefined {
    return toValue(source)?.[key];
  }

  function write(key: keyof T, value: unknown): void {
    // 构造入口约束字段类型。USelect 事件将枚举值扩宽为 string，在此构造字段 patch。
    edit({ [key]: value } as Partial<T>);
  }

  function switchField(
    key: FieldKey<T, boolean>,
    options: SettingsItemOptions = {},
  ): SwitchSettingsItem {
    return {
      ...defaults,
      ...options,
      kind: 'switch',
      read: (): boolean | undefined => read(key) as boolean | undefined,
      write: (value: boolean): void => write(key, value),
    };
  }

  function slider(key: FieldKey<T, number>, options: SliderOptions): SliderSettingsItem {
    return {
      ...defaults,
      ...options,
      kind: 'slider',
      read: (): number | undefined => read(key) as number | undefined,
      write: (value: number): void => write(key, value),
    };
  }

  function select<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
    options: SettingsItemOptions = {},
  ): SelectSettingsItem {
    return {
      ...defaults,
      ...options,
      kind: 'select',
      items,
      read: (): string | undefined => read(key) as string | undefined,
      write: (value: string): void => write(key, value),
    };
  }

  function input(key: FieldKey<T, string | null>, options: InputOptions = {}): InputSettingsItem {
    return {
      ...defaults,
      ...options,
      kind: 'input',
      read: (): string | null | undefined => read(key) as string | null | undefined,
      write: (value: string): void => write(key, value),
    };
  }

  function choices<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
    options: SettingsItemOptions = {},
  ): ChoiceButtonsSettingsItem {
    return {
      ...defaults,
      ...options,
      kind: 'choice-buttons',
      items,
      read: (): string | undefined => read(key) as string | undefined,
      write: (value: string): void => write(key, value),
    };
  }

  return { switch: switchField, slider, select, input, choices };
}
