# 声明式设置项原型

本原型用于验证：页面只声明字段、控件选项与可用性，能否隐藏重复的控件绑定和状态展示，同时保留现有行为。

代码保存在 `van/prototype/declarative-settings` 本地分支。`settings.vue` 已迁移到该接口，旁边的 `settings-prototype.vue` 提供内存演示。原型路由属于本次评估材料，准备合入主线时应移除或另行决定保留方式。

## 运行与体验

在仓库根目录运行：

```bash
pnpm dev
```

打开 <http://localhost:1420/settings-prototype>。如果 macOS 报文件监听 `EMFILE`，可用 `CHOKIDAR_USEPOLLING=1 pnpm dev` 启用轮询。

演示页使用真实 `createSettingsStore`，加载、保存与加密 IO 使用内存实现。保存延迟 600ms，检查更新延迟 900ms。它不读取或写入用户设置文件。语言切换会更新当前应用界面，刷新后重新初始化。

建议按下面顺序体验，右侧显示当前 `draft`、`effective`、保存错误和成功保存次数：

1. 正常状态下修改开关或音量，观察草稿先变化，生效值在保存完成后变化。
2. 勾选“保存失败”并修改设置，观察草稿与生效值分离。取消勾选，再点击“重试保存”。
3. 选择“加载中”“加载失败”“浏览器不可用”，观察语言按钮、持久化控件、本地缩放和检查更新各自的展示规则。
4. 选择“CDK 无法解密”，聚焦再失焦。原密文仍保留，只有编辑内容或显式清除才会替换它。
5. 选择自定义代理，输入地址。输入期间不提交，失焦或 Enter 后保存。

实际设置页仍在 `/settings`，使用原有 store 与业务操作。纯浏览器模式仍提示后端设置不可用。OEA dev 的在线更新限制继续生效，演示页单独模拟这些控件，方便本地检查。

## 页面接口

持久化设置的数据源在页面连接一次：

```ts
const fields = useSettingsFields<DraftSettings>(draftSettings, editSettings);
const controls = {
  tray: fields.switch('minimizeToTray'),
  volume: fields.slider('soundVolume', {
    min: 0,
    max: 1,
    step: 0.05,
    formatValue: (value: number) => n(value, 'percent'),
  }),
};
```

模板声明完整设置项：

```vue
<SettingsItem
  :control="controls.tray"
  :description="t('settings.interface.tray.description')"
  icon="i-lucide-panel-bottom-close"
  :state="persistedItemState"
  :title="t('settings.interface.tray.title')"
/>
```

`state` 有四种状态：`ready` 展示控件，`disabled` 保留控件并禁用，`loading` 按控件种类展示 skeleton，`unavailable` 用带原因的 badge 替换控件。页面决定状态，组件决定如何展示。不可用 badge 的 variant 和 skeleton 种类可覆盖。

独立数据源使用相同的控件描述接口。例如本地缩放提供 `read` 和 `write`，动作按钮提供 `onClick`、`loading` 和 `disabled` 的响应式 getter。共享组件不需要知道数据来自全局 store、本地 ref 还是异步操作。

复合设置项继续使用插槽，内部可复用 `SettingsControl`。CDK 的警告、清除操作和订阅链接仍留在页面。原有 `SettingsItem` 插槽用法继续可用。自定义插槽可读取 `disabled` slot prop，任意后代不会自动继承禁用状态。

## 内部职责

- `settingsItem.ts` 定义控件描述、展示状态和 skeleton 尺寸。它是组件与 composable 共享的接口。
- `useSettingsFields` 将只读数据投影和 patch 编辑入口绑定为字段描述。字段名与选项值受类型约束，每次读取都访问最新投影。
- `useSettingsControl` 将控件描述适配为响应式值和 Nuxt UI 事件，统一 lazy 输入、Enter 失焦和滑块数值提交。
- `SettingsControl.vue` 集中创建 `USwitch`、`USlider`、`USelect`、`UInput` 和 `UButton`。
- `SettingsItem.vue` 集中处理正常内容、禁用、加载占位与不可用原因。

两个 composable 通过控件描述连接，各自不调用对方，也不导入业务 store。设置规范化、加密、保存队列和 `effective` 发布时机仍由现有 `settingsStore` 负责。

## 验证结果与范围

已通过 `pnpm fix`、`pnpm build`、`pnpm check`，以及 `pnpm test src/features/settings` 的 16 个现有测试。构建和测试在本机使用 `CHOKIDAR_USEPOLLING=1` 绕过文件监听限制。

浏览器交互已验证保存失败与重试、CDK 普通失焦与显式清除、各可用性场景、检查更新的忙碌禁用、保存后切换语言、自定义代理显隐、lazy 输入与 Enter 提交、滑块键盘编辑。控件适配另外验证了数组形式的滑块回写被忽略。

实际 `/settings` 的浏览器不可用布局与改动前截图一致。本地演示覆盖控件及真实前端 store，未验证 Windows 桌面 IPC、真实加密或真实在线更新服务。

## 当前判断

这个接口能够收拢普通设置项的字段绑定、控件事件和展示状态，页面保留业务条件与复合内容。它不要求引入表单框架或 schema 表达式系统。正式采用前主要需要评估字段描述的阅读体验，以及自定义插槽是否仍足够直接。
