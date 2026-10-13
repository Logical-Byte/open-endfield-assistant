import { computed, toValue, type ComputedRef, type WritableComputedRef } from 'vue';
import type { SettingsChoice, SettingsItemDefinition } from './settingsItem';

/** 各种字段可供控件显示的值。动作按钮没有字段值。 */
type SettingsControlValue = string | number | boolean | null | undefined;

/** Nuxt UI 渲染器使用的响应式投影与事件入口。 */
export interface SettingsControlView {
  /** 最新字段值。按钮返回 undefined。 */
  value: ComputedRef<SettingsControlValue>;
  /** 文本输入的读写投影。空显示不会自动清除业务中的 null。 */
  inputValue: WritableComputedRef<string>;
  /** 转为 Nuxt UI 接受的可变数组，保留调用方选项的只读约定。 */
  items: ComputedRef<SettingsChoice[]>;
  /** 输入占位文案，随描述中的 ref/getter 更新。 */
  placeholder: ComputedRef<string | undefined>;
  /** 动作按钮文案，随应用语言更新。 */
  buttonLabel: ComputedRef<string | undefined>;
  /** 动作自身是否禁用，渲染器再与整项禁用状态合并。 */
  buttonDisabled: ComputedRef<boolean>;
  /** 动作自身是否忙碌，不代表设置项处于初始加载。 */
  buttonLoading: ComputedRef<boolean>;
  /** 将开关编辑写入绑定入口。 */
  updateSwitch(value: boolean): void;
  /** 仅提交标量数值，忽略 USlider 的数组和 undefined 回写。 */
  updateSlider(value: number | number[] | undefined): void;
  /** 下拉框和按钮组共享的选项提交入口。 */
  updateChoice(value: string): void;
  /** Enter 触发输入框失焦，沿用 UInput.lazy 的 change 提交。 */
  blurOnEnter(event: KeyboardEvent): void;
}

/**
 * 将设置项描述适配为 Nuxt UI 控件所需的响应式值和事件。
 *
 * item 使用 getter，使组件收到新的描述时也能读取最新连接。
 * 本 composable 只负责控件交互，不管理整项展示状态、不持有业务设置，
 * 也不调用字段工厂或业务 store。输入中间文本由 UInput 的 lazy 模式处理。
 */
export function useSettingsControl(item: () => SettingsItemDefinition): SettingsControlView {
  const value: ComputedRef<SettingsControlValue> = computed((): SettingsControlValue => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'button' ? undefined : current.read();
  });
  const inputValue: WritableComputedRef<string> = computed({
    get(): string {
      const current: SettingsItemDefinition = item();
      return current.kind === 'input' ? (current.read() ?? '') : '';
    },
    set(next: string): void {
      const current: SettingsItemDefinition = item();
      if (current.kind === 'input') current.write(next);
    },
  });
  const items: ComputedRef<SettingsChoice[]> = computed((): SettingsChoice[] => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'select' || current.kind === 'choice-buttons'
      ? [...toValue(current.items)]
      : [];
  });
  const placeholder: ComputedRef<string | undefined> = computed((): string | undefined => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'input' ? toValue(current.placeholder) : undefined;
  });
  const buttonLabel: ComputedRef<string | undefined> = computed((): string | undefined => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'button' ? toValue(current.label) : undefined;
  });
  const buttonDisabled: ComputedRef<boolean> = computed((): boolean => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'button' && !!toValue(current.disabled);
  });
  const buttonLoading: ComputedRef<boolean> = computed((): boolean => {
    const current: SettingsItemDefinition = item();
    return current.kind === 'button' && !!toValue(current.loading);
  });

  function updateSwitch(next: boolean): void {
    const current: SettingsItemDefinition = item();
    if (current.kind === 'switch') current.write(next);
  }

  function updateSlider(next: number | number[] | undefined): void {
    const current: SettingsItemDefinition = item();
    // USlider 会短暂回写数组，保持页面原有的仅接收 number 的行为。
    if (current.kind === 'slider' && typeof next === 'number') current.write(next);
  }

  function updateChoice(next: string): void {
    const current: SettingsItemDefinition = item();
    if (current.kind === 'select' || current.kind === 'choice-buttons') current.write(next);
  }

  function blurOnEnter(event: KeyboardEvent): void {
    (event.target as HTMLInputElement).blur();
  }

  return {
    value,
    inputValue,
    items,
    placeholder,
    buttonLabel,
    buttonDisabled,
    buttonLoading,
    updateSwitch,
    updateSlider,
    updateChoice,
    blurOnEnter,
  };
}
