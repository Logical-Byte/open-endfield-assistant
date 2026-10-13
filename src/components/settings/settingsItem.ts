import type { BadgeProps, ButtonProps } from '@nuxt/ui';
import type { MaybeRefOrGetter } from 'vue';

/** 展示状态由调用方决定，设置项不读取任何业务 store。 */
export type SettingsItemState =
  | { status: 'ready' }
  | { status: 'loading' }
  | { status: 'disabled' }
  | { status: 'unavailable'; label: string; variant?: BadgeProps['variant'] };

interface ValueControl<T> {
  read(): T | undefined;
  write(value: T): void;
}

export interface SettingsChoice<T extends string = string> {
  label: string;
  value: T;
}

export interface SwitchControl extends ValueControl<boolean> {
  kind: 'switch';
}

export interface SliderOptions {
  min: number;
  max: number;
  step: number;
  formatValue(value: number): string;
  marks?: readonly number[];
  tooltip?: boolean;
}

export interface SliderControl extends ValueControl<number>, SliderOptions {
  kind: 'slider';
}

export interface SelectControl extends ValueControl<string> {
  kind: 'select';
  items: MaybeRefOrGetter<readonly SettingsChoice[]>;
}

export interface InputOptions {
  type?: 'text' | 'password';
  placeholder?: MaybeRefOrGetter<string>;
}

export interface InputControl extends InputOptions {
  kind: 'input';
  read(): string | null | undefined;
  write(value: string): void;
}

export interface ChoiceButtonsControl extends ValueControl<string> {
  kind: 'choice-buttons';
  items: MaybeRefOrGetter<readonly SettingsChoice[]>;
}

export interface ButtonControl {
  kind: 'button';
  label: MaybeRefOrGetter<string>;
  icon?: string;
  variant?: ButtonProps['variant'];
  color?: ButtonProps['color'];
  disabled?: MaybeRefOrGetter<boolean>;
  loading?: MaybeRefOrGetter<boolean>;
  onClick?: () => void | Promise<void>;
}

export type SettingsControl =
  | SwitchControl
  | SliderControl
  | SelectControl
  | InputControl
  | ChoiceButtonsControl
  | ButtonControl;

export type SettingsSkeleton = SettingsControl['kind'];

export const settingsSkeletonClasses: Record<SettingsSkeleton, string> = {
  switch: 'h-5 w-10 rounded-full',
  slider: 'h-5 w-56',
  select: 'h-8 w-56',
  input: 'h-8 w-56',
  'choice-buttons': 'h-8 w-56',
  button: 'h-8 w-32',
};
