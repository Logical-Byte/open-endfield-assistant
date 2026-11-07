use super::*;
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::resizable::{h_resizable, resizable_panel};

impl Oea {
    pub(super) fn choice_menu(
        &self,
        id: &'static str,
        title: impl Into<SharedString>,
        choices: Vec<(SharedString, Action, bool)>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let view = cx.entity().downgrade();
        Button::new(id)
            .label(title)
            .icon(IconName::ChevronDown)
            .small()
            .dropdown_menu(move |mut menu, _, cx| {
                for (title, action, checked) in &choices {
                    let view = view.clone();
                    let action = *action;
                    menu = menu.item(
                        PopupMenuItem::new(i18n::label(title, cx))
                            .checked(*checked)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |view, cx| view.action(action, window, cx));
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }
    pub(super) fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        action: Action,
        active: bool,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(if matches!(action, Action::Assign(..)) {
                label.into()
            } else {
                i18n::label(label.into().as_ref(), cx)
            })
            .small()
            .when(active, |button| button.primary())
            .on_click(cx.listener(move |view, _, window, cx| view.action(action, window, cx)))
    }

    pub(super) fn panel(&self, cx: &App) -> Div {
        div()
            .flex()
            .flex_col()
            .h_full()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_xl()
    }

    pub(super) fn sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let mut nav = div().flex().flex_col().gap_1().px_2();
        for (i, (page, icon, title)) in [
            (Page::Scan, IconName::Frame, "档案扫描"),
            (Page::Battle, IconName::Asterisk, "百趣演武"),
            (Page::Logs, IconName::File, "日志"),
            (Page::Monitor, IconName::Eye, "监控"),
        ]
        .into_iter()
        .enumerate()
        {
            nav = nav.child(
                div()
                    .id(("nav", i))
                    .flex()
                    .items_center()
                    .gap_3()
                    .h_10()
                    .px_3()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(if self.page == page {
                        cx.theme().primary
                    } else {
                        cx.theme().foreground
                    })
                    .when(self.page == page, |el| el.bg(cx.theme().accent))
                    .hover(|el| el.bg(cx.theme().accent))
                    .child(Icon::new(icon).size_4())
                    .when(!self.collapsed, |el| el.child(label(title, cx)))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.action(Action::Navigate(page), window, cx)
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .h_full()
            .w(px(if self.collapsed { 56. } else { 176. }))
            .rounded_xl()
            .bg(cx.theme().sidebar)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .h(px(56.))
                    .flex()
                    .items_center()
                    .px_3()
                    .gap_2()
                    .child(
                        img(Arc::new(gpui::Image::from_bytes(
                            gpui::ImageFormat::Png,
                            include_bytes!("../../../../resources/icons/icon.png").to_vec(),
                        )))
                        .size_6(),
                    )
                    .when(!self.collapsed, |el| {
                        el.child(div().font_semibold().child(product_name()))
                            .child(muted(
                                if matches!(
                                    crate::app_variant::CURRENT,
                                    crate::app_variant::AppVariant::Dev
                                ) {
                                    ""
                                } else {
                                    env!("OEA_VERSION")
                                },
                                cx,
                            ))
                    }),
            )
            .child(nav)
            .child(div().flex_1())
            .when(!self.collapsed, |el| {
                el.child(
                    div()
                        .px_3()
                        .pb_3()
                        .child(muted("Open Endfield Assistant", cx)),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .p_2()
                    .child(
                        self.button(
                            "settings",
                            "⚙",
                            Action::Navigate(Page::Settings),
                            self.page == Page::Settings,
                            cx,
                        )
                        .tooltip("设置"),
                    )
                    .when(!self.collapsed, |el| {
                        el.child(
                            self.button(
                                "help",
                                "?",
                                Action::Navigate(Page::Help),
                                self.page == Page::Help,
                                cx,
                            )
                            .tooltip("帮助"),
                        )
                        .child(
                            self.button(
                                "theme",
                                if cx.theme().is_dark() { "☾" } else { "☼" },
                                Action::Theme,
                                false,
                                cx,
                            )
                            .tooltip("切换明暗主题"),
                        )
                    })
                    .child(
                        self.button(
                            "collapse",
                            if self.collapsed { "›" } else { "‹" },
                            Action::Collapse,
                            false,
                            cx,
                        )
                        .tooltip("展开或收起侧栏"),
                    ),
            )
    }

    pub(super) fn scan_page(&self, cx: &Context<Self>) -> impl IntoElement {
        h_resizable("archive-workspace")
            .child(
                resizable_panel()
                    .size(px(450.))
                    .size_range(px(300.)..px(1600.))
                    .child(self.archive_panel(cx)),
            )
            .child(
                resizable_panel()
                    .size(px(370.))
                    .size_range(px(320.)..px(1600.))
                    .child(self.scan_panel(cx)),
            )
    }

    pub(super) fn archive_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let collected: HashSet<_> = self.records.iter().filter_map(|r| r.target).collect();
        let total = self.archives.len();
        let query = self.archive_search.read(cx).value().to_lowercase();
        let visible: Vec<_> = self
            .archives
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                (match self.archive_filter {
                    0 => !collected.contains(i),
                    1 => collected.contains(i),
                    _ => true,
                }) && (self.category == 0 || a.category == CATEGORIES[self.category])
                    && (!self.map_only || a.map)
                    && a.title.to_lowercase().contains(&query)
            })
            .collect();
        let mut tabs = div().flex().gap_1();
        for (i, (label, count)) in [
            ("未收集", total - collected.len()),
            ("已收集", collected.len()),
            ("全部", total),
        ]
        .into_iter()
        .enumerate()
        {
            tabs = tabs.child(
                self.button(
                    ("archive-filter", i),
                    format!("{} {count}", i18n::label(label, cx)),
                    Action::ArchiveFilter(i),
                    self.archive_filter == i,
                    cx,
                )
                .xsmall(),
            );
        }
        let mut rows = div()
            .id("archive-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        for (i, archive) in &visible {
            let i = *i;
            let expanded = self.expanded.contains(&i);
            rows = rows.child(
                div()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .id(("archive", i))
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_h_10()
                            .px_3()
                            .py_2()
                            .cursor_pointer()
                            .hover(|el| el.bg(cx.theme().muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .child(archive.title.clone()),
                            )
                            .when(collected.contains(&i), |el| {
                                el.child(
                                    Icon::new(IconName::CircleCheck)
                                        .size_4()
                                        .text_color(rgb(0x22c55e)),
                                )
                            })
                            .child(
                                Icon::new(if expanded {
                                    IconName::ChevronUp
                                } else {
                                    IconName::ChevronDown
                                })
                                .size_3()
                                .text_color(cx.theme().muted_foreground),
                            )
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.action(Action::Expand(i), window, cx)
                            })),
                    )
                    .when(expanded, |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .bg(cx.theme().muted)
                                .when_some(archive.content.clone(), |el, content| {
                                    el.child(muted("正文首行", cx)).child(content)
                                })
                                .child(muted("档案分类", cx))
                                .child(archive.category.clone())
                                .child(muted("获取方式", cx))
                                .child(archive.acquisition.clone())
                                .child(muted("收集状态", cx))
                                .child(if collected.contains(&i) {
                                    "已收集"
                                } else {
                                    "未收集"
                                }),
                        )
                    }),
            );
        }
        if visible.is_empty() {
            rows = rows.child(empty("没有符合条件的档案", cx));
        }
        self.panel(cx)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .h_8()
                            .items_center()
                            .justify_between()
                            .child(heading(format!("{}  {total}", label("档案目录", cx)), cx))
                            .child(
                                self.button("export", "导出到地图集", Action::Export, false, cx)
                                    .xsmall(),
                            ),
                    )
                    .child(
                        div().flex().items_center().gap_1().child(tabs).child(
                            self.button("map-only", "地图拾取", Action::MapOnly, self.map_only, cx)
                                .xsmall(),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.archive_search).small()),
                            )
                            .child(
                                self.choice_menu(
                                    "category",
                                    CATEGORIES[self.category],
                                    CATEGORIES
                                        .iter()
                                        .enumerate()
                                        .map(|(i, title)| {
                                            (
                                                (*title).into(),
                                                Action::Category(i),
                                                i == self.category,
                                            )
                                        })
                                        .collect(),
                                    cx,
                                ),
                            ),
                    ),
            )
            .child(rows)
            .child(
                self.button(
                    "copy-collected",
                    "复制收集结果",
                    Action::CopyCollected,
                    false,
                    cx,
                )
                .xsmall(),
            )
            .child(footer(
                format!("显示 {} / {total} 个档案", visible.len()),
                cx,
            ))
    }

    pub(super) fn scan_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let matched = self.records.iter().filter(|r| r.target.is_some()).count();
        let query = self.scan_search.read(cx).value().to_lowercase();
        let mut tabs = div().flex().gap_1();
        for (i, (label, count)) in [
            ("待确认", self.records.len() - matched),
            ("已匹配", matched),
            ("全部", self.records.len()),
        ]
        .into_iter()
        .enumerate()
        {
            tabs = tabs.child(
                self.button(
                    ("scan-filter", i),
                    format!("{} {count}", i18n::label(label, cx)),
                    Action::ScanFilter(i),
                    self.scan_filter == i,
                    cx,
                )
                .xsmall(),
            );
        }
        let mut list = div()
            .id("scan-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_2()
            .flex()
            .flex_col()
            .gap_2();
        let mut count = 0;
        for (i, record) in self.records.iter().enumerate() {
            if !(match self.scan_filter {
                0 => record.target.is_none(),
                1 => record.target.is_some(),
                _ => true,
            }) || !record
                .observation
                .ocr_result
                .to_lowercase()
                .contains(&query)
            {
                continue;
            }
            count += 1;
            let mut card = div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(muted(
                            format!(
                                "#{}  {}",
                                record.id,
                                self.category_label(record.observation.found_in_category)
                            ),
                            cx,
                        ))
                        .child(
                            div()
                                .text_xs()
                                .text_color(if record.target.is_some() {
                                    rgb(0x22c55e)
                                } else {
                                    rgb(0xf59e0b)
                                })
                                .child(
                                    if record.manual_choice.is_some() && record.target.is_none() {
                                        "人工清除"
                                    } else if record.manual_choice.is_some() {
                                        "人工匹配"
                                    } else if matches!(
                                        record.observation.matching,
                                        archive_scan::MatchResult::Exact { .. }
                                    ) {
                                        "精确匹配"
                                    } else if record.target.is_some() {
                                        "自动采纳"
                                    } else {
                                        "待确认"
                                    },
                                ),
                        ),
                )
                .child(
                    div()
                        .font_medium()
                        .child(record.observation.ocr_result.clone()),
                )
                .child(muted("原始 OCR 识别结果", cx))
                .when_some(record.observation.body_ocr_result.clone(), |el, body| {
                    el.child(muted("正文 OCR", cx)).child(body)
                })
                .when_some(record.target, |el, target| {
                    el.child(
                        div()
                            .text_sm()
                            .child(format!("已匹配：{}", self.archives[target].title)),
                    )
                })
                .child(
                    div().flex().justify_end().child(
                        self.button(
                            ("edit", i),
                            if self.editing == Some(record.id) {
                                "收起"
                            } else {
                                "编辑匹配"
                            },
                            Action::Edit(record.id),
                            false,
                            cx,
                        )
                        .xsmall(),
                    ),
                );
            if self.editing == Some(record.id) {
                let match_query = self.match_search.read(cx).value().to_lowercase();
                card = card.child(Input::new(&self.match_search).small()).child(
                    self.button(
                        ("unassign", i),
                        "清除匹配",
                        Action::Assign(record.id, None),
                        false,
                        cx,
                    )
                    .xsmall(),
                );
                for (target, archive) in self
                    .archives
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| {
                        a.category_id == record.observation.found_in_category
                            && a.title.to_lowercase().contains(&match_query)
                    })
                    .take(8)
                {
                    card = card.child(
                        self.button(
                            ("candidate", target),
                            archive.title.clone(),
                            Action::Assign(record.id, Some(target)),
                            false,
                            cx,
                        )
                        .xsmall()
                        .tooltip(
                            archive
                                .content
                                .clone()
                                .unwrap_or_else(|| archive.title.clone()),
                        ),
                    );
                }
            }
            let record_id = record.id;
            card = card.when_some(record.image.clone(), |el, image| {
                el.child(
                    div()
                        .id(("screenshot", i))
                        .cursor_pointer()
                        .child(
                            img(image)
                                .w_full()
                                .h(px(120.))
                                .object_fit(ObjectFit::Contain),
                        )
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.action(Action::Preview(record_id), window, cx)
                        })),
                )
            });
            list = list.child(card);
        }
        if count == 0 {
            list = list.child(empty(
                if self.records.is_empty() {
                    "点击 “开始扫描” 开始体验"
                } else {
                    "没有符合条件的扫描记录"
                },
                cx,
            ));
        }
        self.panel(cx)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .h_8()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(heading(
                                format!("{}  {}", label("扫描记录", cx), self.records.len()),
                                cx,
                            ))
                            .child(self.button(
                                "scan",
                                if self.status.is_active() {
                                    "停止扫描"
                                } else {
                                    "开始扫描"
                                },
                                Action::Scan,
                                true,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(tabs)
                            .child(
                                self.button("undo", "撤销", Action::Undo, false, cx)
                                    .xsmall()
                                    .disabled(self.undo.is_empty()),
                            )
                            .child(
                                self.button("clear-scans", "清空", Action::ClearScans, false, cx)
                                    .disabled(self.status.is_active())
                                    .xsmall(),
                            ),
                    )
                    .child(Input::new(&self.scan_search).small()),
            )
            .child(list)
            .child(footer(
                format!("显示 {count} / {} 条扫描记录", self.records.len()),
                cx,
            ))
    }
}

pub(super) const CATEGORIES: [&str; 7] = [
    "全部分类",
    "纸质记录",
    "电子档案",
    "藏品",
    "多媒体",
    "中枢档案",
    "调查报告",
];

pub(super) fn heading(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_lg()
        .font_semibold()
        .child(label(text.into().as_ref(), cx))
}
pub(super) fn muted(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(label(text.into().as_ref(), cx))
}
pub(super) fn footer(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(cx.theme().border)
        .child(muted(text, cx))
}
pub(super) fn empty(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .min_h_40()
        .gap_3()
        .child(
            Icon::new(IconName::Inbox)
                .size_8()
                .text_color(cx.theme().muted_foreground),
        )
        .child(muted(text, cx))
}

impl Render for Oea {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Page::Scan => {
                if self.guide {
                    self.guide_page(cx).into_any_element()
                } else {
                    self.scan_page(cx).into_any_element()
                }
            }
            Page::Battle => self.battle_page(cx).into_any_element(),
            Page::Logs => self.logs_page(cx).into_any_element(),
            Page::Monitor => self.monitor_page(cx).into_any_element(),
            Page::Settings => self.settings_page(cx).into_any_element(),
            Page::Help => self.help_page(cx).into_any_element(),
        };
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .text_sm()
            .text_color(cx.theme().foreground)
            .bg(cx.theme().background)
            .when_some(self.error.clone(), |el, error| {
                el.child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .p_3()
                        .bg(rgb(0x7f1d1d))
                        .text_color(rgb(0xffffff))
                        .child(div().flex_1().child(error))
                        .when(self.backend.is_none(), |el| {
                            el.child(self.button("retry", "重试初始化", Action::Retry, false, cx))
                        }),
                )
            })
            .when(self.backend.is_none() && self.error.is_none(), |el| {
                el.child(div().p_3().child(label("正在加载 OCR 模型与档案目录…", cx)))
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .p_2()
                    .child(self.sidebar(cx))
                    .child(div().flex_1().min_w_0().h_full().child(content)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_6()
                    .px_3()
                    .flex_shrink_0()
                    .child(muted(
                        format!(
                            "{}    {} {}    {}",
                            label(
                                match self.status {
                                    automation::Status::Idle { .. } => "已就绪",
                                    automation::Status::Running { .. } => "正在运行",
                                    automation::Status::Stopping { .. } => "正在停止",
                                },
                                cx
                            ),
                            self.records.len(),
                            label("条扫描记录", cx),
                            self.notice.as_deref().unwrap_or("")
                        ),
                        cx,
                    )),
            )
            .when_some(self.preview.clone(), |el, image| {
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(cx.theme().background)
                        .flex()
                        .flex_col()
                        .p_4()
                        .gap_3()
                        .child(self.button(
                            "close-preview",
                            "关闭预览",
                            Action::ClosePreview,
                            false,
                            cx,
                        ))
                        .child(img(image).size_full().object_fit(ObjectFit::Contain)),
                )
            })
            .when(
                self.backend
                    .as_ref()
                    .is_some_and(|backend| backend.updates.is_installing()),
                |el| {
                    el.child(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(cx.theme().background)
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_4()
                            .child(heading("正在安装更新", cx))
                            .child(muted(
                                match self.install_stage {
                                    Some(update::install::InstallStage::Preparing) => "正在准备",
                                    Some(update::install::InstallStage::Extracting) => "正在解压",
                                    Some(update::install::InstallStage::ApplyingIncremental) => {
                                        "正在应用增量更新"
                                    }
                                    Some(update::install::InstallStage::ApplyingFull) => {
                                        "正在应用完整更新"
                                    }
                                    Some(update::install::InstallStage::CleaningUp) => "正在清理",
                                    None => "请稍候",
                                },
                                cx,
                            )),
                    )
                },
            )
    }
}

pub(super) fn configure_theme(cx: &mut App) {
    let dark = cx.theme().is_dark();
    let theme = Theme::global_mut(cx);
    theme.font_size = px(14.);
    theme.primary = rgb(0xec4899).into();
    theme.primary_hover = rgb(0xdb2777).into();
    theme.primary_foreground = rgb(0xffffff).into();
    theme.background = rgb(if dark { 0x18181b } else { 0xffffff }).into();
    theme.sidebar = rgb(if dark { 0x27272a } else { 0xfafafa }).into();
    theme.border = rgb(if dark { 0x3f3f46 } else { 0xe4e4e7 }).into();
    theme.accent = rgb(if dark { 0x452234 } else { 0xfce7f3 }).into();
}
