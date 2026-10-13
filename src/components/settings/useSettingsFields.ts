import { toValue, type MaybeRefOrGetter } from 'vue';
import type {
  ChoiceButtonsControl,
  InputControl,
  InputOptions,
  SelectControl,
  SettingsChoice,
  SliderControl,
  SliderOptions,
  SwitchControl,
} from './settingsItem';

type FieldKey<T, Value> = {
  [Key in keyof T]-?: T[Key] extends Value ? Key : never;
}[keyof T];

interface SettingsFields<T extends object> {
  switch(key: FieldKey<T, boolean>): SwitchControl;
  slider(key: FieldKey<T, number>, options: SliderOptions): SliderControl;
  select<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
  ): SelectControl;
  input(key: FieldKey<T, string | null>, options?: InputOptions): InputControl;
  choices<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
  ): ChoiceButtonsControl;
}

/**
 * 数据源只需提供只读投影和 patch 编辑入口。每项按字段名声明连接，
 * 生成的描述不复制字段值，每次读取都连接到最新投影。
 */
export function useSettingsFields<T extends object>(
  source: MaybeRefOrGetter<Readonly<T> | null>,
  edit: (patch: Partial<T>) => void,
): SettingsFields<T> {
  function read<Key extends keyof T>(key: Key): T[Key] | undefined {
    return toValue(source)?.[key];
  }

  function write(key: keyof T, value: unknown): void {
    // 各构造函数约束字段类型。USelect 的字符串接口在这里收敛回字段 patch。
    edit({ [key]: value } as Partial<T>);
  }

  function switchField(key: FieldKey<T, boolean>): SwitchControl {
    return {
      kind: 'switch',
      read: () => read(key) as boolean | undefined,
      write: (value: boolean) => write(key, value),
    };
  }

  function slider(key: FieldKey<T, number>, options: SliderOptions): SliderControl {
    return {
      ...options,
      kind: 'slider',
      read: () => read(key) as number | undefined,
      write: (value: number) => write(key, value),
    };
  }

  function select<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
  ): SelectControl {
    return {
      kind: 'select',
      items,
      read: () => read(key) as string | undefined,
      write: (value: string) => write(key, value),
    };
  }

  function input(key: FieldKey<T, string | null>, options: InputOptions = {}): InputControl {
    return {
      ...options,
      kind: 'input',
      read: () => read(key) as string | null | undefined,
      write: (value: string) => write(key, value),
    };
  }

  function choices<Key extends FieldKey<T, string>>(
    key: Key,
    items: MaybeRefOrGetter<readonly SettingsChoice<Extract<T[Key], string>>[]>,
  ): ChoiceButtonsControl {
    return {
      kind: 'choice-buttons',
      items,
      read: () => read(key) as string | undefined,
      write: (value: string) => write(key, value),
    };
  }

  return { switch: switchField, slider, select, input, choices };
}
