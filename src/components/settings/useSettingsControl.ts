import { computed, toValue, type ComputedRef, type WritableComputedRef } from 'vue';
import type { SettingsChoice, SettingsControl } from './settingsItem';

interface SettingsControlView {
  value: ComputedRef<string | number | boolean | null | undefined>;
  inputValue: WritableComputedRef<string>;
  items: ComputedRef<SettingsChoice[]>;
  placeholder: ComputedRef<string | undefined>;
  buttonLabel: ComputedRef<string | undefined>;
  buttonDisabled: ComputedRef<boolean>;
  buttonLoading: ComputedRef<boolean>;
  updateSwitch(value: boolean): void;
  updateSlider(value: number | number[] | undefined): void;
  updateChoice(value: string): void;
  blurOnEnter(event: KeyboardEvent): void;
}

/** 控件适配只处理展示与输入事件，不持有业务设置，也不调用其他 composable。 */
export function useSettingsControl(control: () => SettingsControl): SettingsControlView {
  const value = computed(() => {
    const current = control();
    return current.kind === 'button' ? undefined : current.read();
  });
  // 输入中间值继续交给 UInput 的 lazy 模式，普通失焦不额外提交。
  const inputValue = computed<string>({
    get() {
      const current = control();
      return current.kind === 'input' ? (current.read() ?? '') : '';
    },
    set(next: string) {
      const current = control();
      if (current.kind === 'input') current.write(next);
    },
  });
  const items = computed(() => {
    const current = control();
    return current.kind === 'select' || current.kind === 'choice-buttons'
      ? [...toValue(current.items)]
      : [];
  });
  const placeholder = computed(() => {
    const current = control();
    return current.kind === 'input' ? toValue(current.placeholder) : undefined;
  });
  const buttonLabel = computed(() => {
    const current = control();
    return current.kind === 'button' ? toValue(current.label) : undefined;
  });
  const buttonDisabled = computed(() => {
    const current = control();
    return current.kind === 'button' && !!toValue(current.disabled);
  });
  const buttonLoading = computed(() => {
    const current = control();
    return current.kind === 'button' && !!toValue(current.loading);
  });

  function updateSwitch(next: boolean): void {
    const current = control();
    if (current.kind === 'switch') current.write(next);
  }

  function updateSlider(next: number | number[] | undefined): void {
    const current = control();
    // USlider 会短暂回写数组，保持页面原有的仅接收 number 的行为。
    if (current.kind === 'slider' && typeof next === 'number') current.write(next);
  }

  function updateChoice(next: string): void {
    const current = control();
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
