import type { BadgeProps, ButtonProps } from '@nuxt/ui';
import type { MaybeRefOrGetter } from 'vue';

/** 设置项右侧的展示状态。由页面判断业务可用性，共享组件负责渲染。 */
export type SettingsItemState =
  | { status: 'ready' }
  | { status: 'loading' }
  | { status: 'disabled' }
  | {
      status: 'unavailable';
      /** 替换控件的不可用原因，随应用语言更新。 */
      label: string;
      /** 原因 badge 的外观，默认 soft。 */
      variant?: BadgeProps['variant'];
    };

/** 所有设置项共享的选项，也用于字段工厂的默认值和逐项覆盖。 */
export interface SettingsItemOptions {
  /** 默认 ready。传入 ref 或 getter 保持可用性响应式，避免传入一次性快照。 */
  state?: MaybeRefOrGetter<SettingsItemState>;
}

/** 字段与控件之间的读写接口。实现方负责保存、规范化和生效时机。 */
interface SettingsValueBinding<T> {
  /** 每次读取最新编辑值。数据尚未加载时可返回 undefined。 */
  read(): T | undefined;
  /** 提交编辑值。持久化设置应写入 patch 入口，不直接修改只读草稿。 */
  write(value: T): void;
}

/** 下拉框和按钮组选项。泛型限定实际字段允许的字符串值。 */
export interface SettingsChoice<T extends string = string> {
  /** 显示文案，通常由调用方翻译。 */
  label: string;
  /** 写回字段的业务值。 */
  value: T;
  /** 按钮右上角的可选标记。 */
  badge?: Pick<BadgeProps, 'label' | 'color'>;
}

/** 布尔字段设置项，loading 时使用开关形状的 skeleton。 */
export interface SwitchSettingsItem extends SettingsValueBinding<boolean>, SettingsItemOptions {
  /** 创建 USwitch。 */
  kind: 'switch';
}

/** 滑块的展示和编辑选项，独立于字段的数据源。 */
export interface SliderOptions extends SettingsItemOptions {
  /** 用户通过滑块可选择的最小值。 */
  min: number;
  /** 用户通过滑块可选择的最大值。 */
  max: number;
  /** 鼠标和键盘编辑的步长，已有设置值不会据此舍入。 */
  step: number;
  /** 格式化当前值及刻度文案，例如将 0.5 显示为 50%。 */
  formatValue(value: number): string;
  /** 可选的静态刻度，显示在滑块下方。 */
  marks?: readonly number[];
  /** 是否显示 Nuxt UI 的滑块 tooltip，默认关闭。 */
  tooltip?: boolean;
}

/** 数值字段设置项，只提交 USlider 发出的标量数值。 */
export interface SliderSettingsItem extends SettingsValueBinding<number>, SliderOptions {
  /** 创建 USlider 并显示格式化后的当前值。 */
  kind: 'slider';
}

/** 字符串选项设置项，选项可通过 getter 随语言或业务状态更新。 */
export interface SelectSettingsItem extends SettingsValueBinding<string>, SettingsItemOptions {
  /** 创建 USelect。 */
  kind: 'select';
  /** 可选业务值与显示文案。 */
  items: MaybeRefOrGetter<readonly SettingsChoice[]>;
}

/** 文本输入的选项。输入过程不提交，change 或 Enter 失焦时提交。 */
export interface InputOptions extends SettingsItemOptions {
  /** 默认 text，CDK 等敏感内容使用 password。 */
  type?: 'text' | 'password';
  /** 占位文案，可传入 getter 随应用语言更新。 */
  placeholder?: MaybeRefOrGetter<string>;
}

/** 文本设置项。null 可表达 CDK 无法解密，显示为空但不会自动写回空字符串。 */
export interface InputSettingsItem extends InputOptions {
  /** 创建使用 lazy 模式的 UInput。 */
  kind: 'input';
  /** 读取最新文本。null 与 undefined 仅在显示时转换为空字符串。 */
  read(): string | null | undefined;
  /** 用户确实编辑文本或显式清除后，提交字符串。 */
  write(value: string): void;
}

/** 少量互斥选项以按钮组呈现，例如应用语言。 */
export interface ChoiceButtonsSettingsItem
  extends SettingsValueBinding<string>, SettingsItemOptions {
  /** 创建 UButton 组，当前值使用 solid，其余值使用 outline。 */
  kind: 'choice-buttons';
  /** 按显示顺序提供选项。 */
  items: MaybeRefOrGetter<readonly SettingsChoice[]>;
}

/** 操作设置项，不绑定字段，也不经过设置保存入口。 */
export interface ButtonSettingsItem extends SettingsItemOptions {
  /** 创建独立的 UButton。 */
  kind: 'button';
  /** 操作文案，可传入翻译 getter。 */
  label: MaybeRefOrGetter<string>;
  /** 完整的图标名称，供构建时离线打包。 */
  icon?: string;
  /** Nuxt UI 按钮变体，省略时使用组件默认值。 */
  variant?: ButtonProps['variant'];
  /** Nuxt UI 语义色，省略时使用组件默认值。 */
  color?: ButtonProps['color'];
  /** 操作自身的禁用条件，与整项 state 的 disabled 合并。 */
  disabled?: MaybeRefOrGetter<boolean>;
  /** 操作自身的忙碌状态，与初始加载 skeleton 分开。 */
  loading?: MaybeRefOrGetter<boolean>;
  /** 点击操作，可返回异步任务。固定展示按钮可省略。 */
  onClick?: () => void | Promise<void>;
}

/** SettingsItem 的统一描述，合并控件连接、选项与展示状态。 */
export type SettingsItemDefinition =
  | SwitchSettingsItem
  | SliderSettingsItem
  | SelectSettingsItem
  | InputSettingsItem
  | ChoiceButtonsSettingsItem
  | ButtonSettingsItem;

/** skeleton 按设置项种类推导，复合内容也可显式指定形状。 */
export type SettingsSkeleton = SettingsItemDefinition['kind'];

/** 保持各种控件在加载前后的占位尺寸一致。 */
export const settingsSkeletonClasses: Record<SettingsSkeleton, string> = {
  switch: 'h-5 w-10 rounded-full',
  slider: 'h-5 w-56',
  select: 'h-8 w-56',
  input: 'h-8 w-56',
  'choice-buttons': 'h-8 w-56',
  button: 'h-8 w-32',
};
