# 设置项组件

`SettingsItem` 声明完整设置项的标题、说明、控件与展示状态。普通字段通过 `useSettingsFields` 连接数据源，独立状态和操作可以直接提供具名设置项描述。共享实现不读取业务 store。

## 普通持久化字段

以下示例中的草稿、编辑入口、加载状态和文案来自页面业务。先连接只读草稿、patch 编辑入口和共同展示状态，再逐项声明字段：

```ts
import { computed } from 'vue';
import { useSettingsFields, type SettingsFields } from './useSettingsFields';
import type { DraftSettings } from '@/features/settings/settingsStore';
import type {
  ButtonSettingsItem,
  ChoiceButtonsSettingsItem,
  SettingsItemState,
  SliderSettingsItem,
  SwitchSettingsItem,
} from './settingsItem';

const persistedState = computed<SettingsItemState>(() => {
  if (loading.value) return { status: 'loading' };
  return draft.value
    ? { status: 'ready' }
    : { status: 'unavailable', label: unavailableLabel.value };
});

const fields: SettingsFields<DraftSettings> = useSettingsFields(draft, edit, {
  state: persistedState,
});
const trayItem: SwitchSettingsItem = fields.switch('minimizeToTray');
```

```vue
<SettingsItem
  :item="trayItem"
  :title="t('settings.interface.tray.title')"
  :description="t('settings.interface.tray.description')"
  icon="i-lucide-panel-bottom-close"
/>
```

`switch` 只接受布尔字段，`slider` 接受数值字段，`select` 和 `choices` 接受字符串字段且约束选项值，`input` 接受字符串或可空字符串字段。每次读取都访问最新草稿，写入构造单字段 patch。字段工厂不管理保存、生效时机或业务规范化。

工厂的 `defaults` 在创建设置项时组合，逐项选项可以覆盖。传入 ref 或 getter 保持动态值响应式，不要在创建时用 `.value` 取出状态快照。

```ts
const languageItem: ChoiceButtonsSettingsItem = fields.choices('uiLocale', languageOptions, {
  state: languageState,
});
const volumeItem: SliderSettingsItem = fields.slider('soundVolume', {
  min: 0,
  max: 1,
  step: 0.05,
  formatValue: (value: number): string => n(value, 'percent'),
});
```

## 展示状态

`item.state` 与控件连接放在同一个描述中。省略时默认为 `ready`：

- `ready`：展示控件或自定义内容。
- `disabled`：保留控件并禁用，适用于需要保留选项的情况。
- `loading`：保留标题和说明，右侧替换为按 `item.kind` 推导的 skeleton。
- `unavailable`：保留标题和说明，右侧替换为原因 badge。`label` 必填，`variant` 默认为 `soft`。

状态可传普通值、ref 或 getter。`SettingsItem` 在 computed 中读取它，状态和翻译更新会重新渲染。`skeleton` prop 可覆盖默认的骨架形状。

## 独立状态和操作

本地状态直接提供 `read/write`。它可以使用与持久化字段不同的展示规则：

```ts
const scaleItem: SliderSettingsItem = {
  kind: 'slider',
  state: scaleState,
  read: (): number => uiScale.value,
  write: (value: number): void => {
    uiScale.value = value;
  },
  min: 0.5,
  max: 2,
  step: 0.05,
  marks: [0.5, 1, 1.5, 2],
  tooltip: true,
  formatValue: (value: number): string => n(value, 'percent'),
};
```

动作按钮没有字段值，直接提供操作入口：

```ts
const checkItem: ButtonSettingsItem = {
  kind: 'button',
  state: checkState,
  label: (): string => t('settings.update.check.button'),
  onClick: checkUpdate,
  disabled: (): boolean => operationBusy.value,
  loading: (): boolean => checking.value,
};
```

按钮自己的 `disabled` 与整项禁用状态合并，`loading` 展示操作进行中的指示。业务模块负责操作互斥和错误处理，控件适配不增加自己的异步状态机。省略 `onClick` 可展示固定按钮，例如当前仅支持一种语言的游戏语言项。

## 复合内容与插槽

`left`、`icon`、`title`、`description` 插槽保留原有布局定制。默认插槽可以替换整个右侧控件区域，例如 CDK 输入、警告和清除按钮。整项加载或不可用时，该区域统一被 skeleton 或 badge 替换。

```vue
<SettingsItem :item="cdkItem" :title="t('settings.update.cdk.title')">
  <template #default="{ disabled }">
    <div class="flex flex-col items-center gap-1">
      <SettingsControl
        :item="cdkItem"
        :disabled="disabled"
        :aria-label="t('settings.update.cdk.title')"
      />
      <UButton
        v-if="draft?.mirrorchyanCdk === null"
        :disabled="disabled"
        :label="t('settings.update.cdk.clear')"
        @click="cdkItem.write('')"
      />
    </div>
  </template>
</SettingsItem>
```

自定义后代需要显式接收 slot prop `disabled`，任意插槽内容不会自动继承禁用状态。`SettingsControl` 在插槽中复用普通控件的输入适配，ARIA 标签由调用方提供。省略 `item` 的纯插槽用法继续可用，例如开发者选项中的特殊工具。

## 输入语义与内部职责

输入框保留 `UInput` 的 lazy 模式，输入过程不写回，change 时提交。Enter 通过失焦触发原有提交行为。普通聚焦和失焦不会额外产生修改。可空字段显示为空，但不会因为显示为空就写回空字符串。

滑块只提交数值，忽略 Nuxt UI 临时回写的数组或 undefined。滑块步长只影响用户编辑，已有值的业务规范化由数据源处理。

- [settingsItem.ts](settingsItem.ts)：统一描述、具体具名类型、字段约定和骨架尺寸。
- [useSettingsFields.ts](useSettingsFields.ts)：从数据源生成具名字段设置项，并组合默认状态和逐项选项。
- [useSettingsControl.ts](useSettingsControl.ts)：把描述适配为 Nuxt UI 的响应式显示值和输入事件。
- [SettingsControl.vue](SettingsControl.vue)：集中创建具体 Nuxt UI 控件，供普通设置项和复合插槽使用。
- [SettingsItem.vue](SettingsItem.vue)：处理行布局、可用性展示和插槽。

两个 composable 通过设置项描述连接，各自不调用对方。新增业务规则通常只需调整页面声明。新增控件种类需要扩展描述类型、字段工厂（若绑定字段）、输入适配、渲染分支和骨架尺寸。实际用法见 [settings.vue](../../pages/settings.vue)。
