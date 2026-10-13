# 设置项声明式界面调研

调研日期：2026-10-13。源码链接以调研时各项目主分支为主，后续可能变化。本文基于公开文档与源码检查，未运行外部项目。

目标：比较公开项目如何收拢设置项的标题、说明、控件、字段绑定和状态展示，让页面主要声明业务连接。本文是接口设计参考，未修改应用实现，也未安装调研涉及的依赖。

## 当前问题与判断标准

OEA 的 `settings.vue` 多次重复 `SettingsItem`、正常控件、加载 skeleton 和不可用 badge。普通持久化设置共享加载规则，手动检查更新和 UI 缩放有独立规则。

调研分别检查三个问题：哪些行为由共享实现负责，调用方仍需声明什么，是否实际覆盖加载占位和不可用原因。控件禁用、异步提交 loading 和初始加载 skeleton 是不同的行为。

## Vue Vben Admin：真实设置页的两种层次

真实 preferences 界面提供 `switch-item.vue`，集中行布局、提示、禁用样式、整行点击切换和底层 Switch。内置设置页仍显式声明模型绑定与禁用条件。[SwitchItem 源码](https://github.com/vbenjs/vue-vben-admin/blob/main/packages/effects/layouts/src/widgets/preferences/blocks/switch-item.vue)、[侧栏设置页](https://github.com/vbenjs/vue-vben-admin/blob/main/packages/effects/layouts/src/widgets/preferences/blocks/layout/sidebar.vue)

更接近 OEA 目标的是项目扩展 preferences：调用方声明 `key`、`component`、`label`、`tip`、选项和控件属性，共享 `custom.vue` 遍历字段，按 switch、number、select、input 分派，并统一值的读取、修改、空值归一化和相关文案翻译。这里集中字段连接，页面不必为每项提供修改回调。[实际字段声明](https://github.com/vbenjs/vue-vben-admin/blob/main/playground/src/preferences.ts)、[扩展设置官方说明](https://doc.vben.pro/en/guide/essentials/settings.html#extend-project-level-preferences)、[扩展设置渲染器](https://github.com/vbenjs/vue-vben-admin/blob/main/packages/effects/layouts/src/widgets/preferences/blocks/custom/custom.vue)

所查设置项实现没有 loading skeleton 或不可用原因 badge。扩展渲染器的四类输入也未覆盖独立动作按钮和 slider。

## Vue Vben Admin：共享表单适配

官方 `Vben Form` 接口用 `fieldName`、`component`、`label` 和 `componentProps` 描述字段。共享适配层注册 UI 模块，并统一各控件的模型属性名称。字段依赖可以统一解析为显隐、禁用和其他属性。项目还保留字段插槽，用于需要自定义内容的情况。[Vben Form 官方文档](https://doc.vben.pro/en/components/common-ui/vben-form.html)、[依赖接口迁移说明](https://doc.vben.pro/en/guide/in-depth/zod-v4-form-migration.html)

值得借鉴：普通项声明字段名后，由共享实现完成取值和提交连接。UI 库差异集中在适配层，页面不需要反复声明同一类绑定。

适用性判断：Vben Form 还管理表单状态、验证和提交生命周期。OEA 已有设置 store 和自动保存语义，借鉴字段描述与渲染的分工更直接。所查表单文档不能证明它提供 OEA 所需的初始加载 skeleton 或“不可用原因替换控件”的统一行为。

## FormKit：一个声明式入口代表完整输入项

FormKit 的统一输入入口按 `type` 选择实现，并接收标签、帮助说明和输入选项。其 Vue 源码根据输入定义渲染模块或内部 schema。调用方也可使用 schema 声明输入，保留局部结构定制能力。[输入文档](https://formkit.com/essentials/inputs)、[Vue 输入入口源码](https://github.com/formkit/formkit/blob/master/packages/vue/src/FormKit.ts)、[schema 文档](https://formkit.com/essentials/schema)

异步提交时，FormKit 的 form 会统一进入 loading 并禁用输入。这是提交阶段的状态管理，不能直接等同于设置初始化时的 skeleton。[form 文档](https://formkit.com/inputs/form)

值得借鉴：把标题、说明、控件及其常规交互封装成完整的输入项，页面仍可以使用直观的模板声明。模板声明和 schema 数组是可选的调用形式，声明式接口并不要求页面全部改成数据数组。

适用性判断：沿用 Nuxt UI 时，需要由自己的输入项实现相同分工，或适配 FormKit 的自定义输入体系。当前没有验证现成的 Nuxt UI 适配是否符合 OEA 的需求。

## Immich：完整设置行，页面继续连接业务状态

Immich 是 Svelte 应用，作为与框架无关的分工参考。`SettingSwitch.svelte` 接收标题、说明、checked、disabled、onToggle 和 isEdited，集中开关、标签、ARIA 和未保存标记。`SettingInputField.svelte` 按 inputType 分派 text、number、password、color，并处理数字空值、范围裁剪以及密码相关交互。[SettingSwitch 源码](https://github.com/immich-app/immich/blob/main/web/src/lib/components/shared-components/settings/SettingSwitch.svelte)、[SettingInputField 源码](https://github.com/immich-app/immich/blob/main/web/src/lib/components/shared-components/settings/SettingInputField.svelte)

真实 BackupSettings 页面仍逐项声明 bind:value、disabled、label 和 isEdited。共享模块收拢了控件行为，业务条件仍位于调用方。[备份设置页](https://github.com/immich-app/immich/blob/main/web/src/routes/admin/system-settings/BackupSettings.svelte)

所查模块没有初始加载 skeleton 或不可用原因 badge。isEdited 表示编辑状态，不能作为 loading 或 unavailable 的证据。

## JSON Forms：数据、界面描述、渲染器分开

JSON Forms 将数据、JSON Schema、UI schema 和 renderer 集合分开。Vue 的 `useJsonFormsControl` 给渲染器提供派生的字段数据、说明、错误和 enabled 状态，以及字段修改入口。具体渲染器注册一次后，多处字段描述可以复用它。[Vue 包的官方 README](https://github.com/eclipsesource/jsonforms/blob/master/packages/vue/README.md)

UI schema 的规则可以控制 SHOW、HIDE、ENABLE 和 DISABLE。[规则文档](https://jsonforms.io/docs/uischema/rules/)

值得借鉴：字段描述与具体控件实现分离，控件适配集中完成。启用规则作为输入交给共享实现，而不是让每个控件直接读取业务 store。

适用性判断：完整方案需要 schema 和 renderer 体系。所查接口没有证明提供 OEA 的 skeleton 与不可用原因展示。官方 Vue 集成页仍标注 preview，若考虑采用依赖，需要进一步检查具体版本和需求。[Vue 集成说明](https://jsonforms.io/docs/integrations/vue/)

## VS Code：通过设置元数据选择编辑器

VS Code 扩展用设置 ID、类型、说明、枚举和约束描述设置，Settings editor 统一解释这些元数据。枚举可以成为下拉选择，字符串还可以指定单行或多行编辑方式。[官方设置贡献接口](https://code.visualstudio.com/api/references/contribution-points#contributes.configuration)

值得借鉴：设置 ID 用来连接数据，元数据用来描述展示，具体控件创建由共享渲染器处理。普通字段不必携带逐项的读取和修改回调。

适用性判断：这是 VS Code 自己的设置平台，无法作为 OEA 的 Vue 模块直接采用。这里借鉴的是字段与渲染的分工。该文档也不能证明它具有 OEA 的运行时加载和不可用展示策略。

## 对 OEA 接口设计的启示

以下是基于上述项目的设计判断，不是这些项目已经实现的 OEA 接口。

1. `SettingsItem` 可以代表完整设置项，并拥有控件、ARIA 标签、输入提交行为、尺寸和状态展示的共享实现。
2. 普通字段的读取和提交连接可以在所属数据源上统一定义。每项声明字段和控件种类，减少重复回调。独立操作项继续接受动作回调。
3. 可用性策略作为外部输入。共享模块不直接依赖 settings store、更新状态或其他业务 composable。
4. 加载 skeleton、不可用原因和正常内容之间的切换，仍需作为 OEA 自己的展示约定实现。
5. 复合内容保留插槽。CDK 的不可读提示、清除入口和订阅链接有自己的交互，不宜为了统一声明而增加一套表达任意界面的语言。

后续的 [本地原型](declarative-settings-prototype.md) 用普通 switch、slider、异步 action 和 CDK 复合项检验了同一个接口，并迁移设置页进行对照。
