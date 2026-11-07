use super::ui::{empty, footer, heading, muted};
use super::*;
use crate::locale::UiLocale;
use gpui_component::switch::Switch;

const HANDS: [(gamble_battle::HandKind, &str); 8] = [
    (gamble_battle::HandKind::StraightFlush, "同花顺"),
    (gamble_battle::HandKind::Quad, "四条"),
    (gamble_battle::HandKind::Flush, "同花"),
    (gamble_battle::HandKind::FullHouse, "葫芦"),
    (gamble_battle::HandKind::Straight, "顺子"),
    (gamble_battle::HandKind::Triplet, "三条"),
    (gamble_battle::HandKind::TwoPair, "两对"),
    (gamble_battle::HandKind::Pair, "对子"),
];
fn hand_name(hand: gamble_battle::HandKind) -> &'static str {
    HANDS.iter().find(|(h, _)| *h == hand).unwrap().1
}

impl Oea {
    pub(super) fn category_label(&self, id: catalog::Category) -> String {
        self.archives
            .iter()
            .find(|a| a.category_id == id)
            .map(|a| a.category.clone())
            .unwrap_or_default()
    }
    pub(super) fn battle_conditions(&self, cx: &App) -> Result<gamble_battle::Conditions, String> {
        let optional = |input: &Entity<InputState>| {
            let text = input.read(cx).value();
            (!text.trim().is_empty()).then(|| text.to_string())
        };
        let c = gamble_battle::Conditions {
            straight_flush: self.conditions[0],
            quad: self.conditions[1],
            flush: self.conditions[2],
            full_house: self.conditions[3],
            straight: self.conditions[4],
            triplet: self.conditions[5],
            two_pair: self.conditions[6],
            pair: self.conditions[7],
            sum: optional(&self.sum_input)
                .map(|s| {
                    s.parse()
                        .map_err(|_| "战力点之和必须是 1 到 25 的整数".to_owned())
                })
                .transpose()?,
            score: optional(&self.score_input)
                .map(|s| s.parse().map_err(|_| "最低分数必须是正整数".to_owned()))
                .transpose()?,
            pattern: optional(&self.pattern_input),
        };
        c.validate()?;
        Ok(c)
    }

    pub(super) fn battle_page(&self, cx: &Context<Self>) -> impl IntoElement {
        let daily = gamble_battle::daily(chrono::Utc::now());
        let mut controls = div()
            .id("conditions")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_3()
            .gap_3()
            .child(div().font_medium().child(label("停止条件", cx)))
            .child(
                div()
                    .p_2()
                    .rounded_md()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().primary)
                    .child(format!("今日好运组合：{}", hand_name(daily.hand))),
            );
        for (i, (kind, name)) in HANDS.iter().enumerate() {
            let multiplier = daily
                .multipliers
                .iter()
                .find(|m| m.hand == *kind)
                .unwrap()
                .tenths
                * (if daily.hand == *kind { 2 } else { 1 });
            controls = controls.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Switch::new(("hand", i))
                            .small()
                            .label(label(*name, cx))
                            .checked(self.conditions[i])
                            .disabled(self.status.is_active())
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.action(Action::Hand(i), window, cx)
                            })),
                    )
                    .child(muted(format!("×{}", multiplier as f32 / 10.), cx)),
            );
        }
        controls = controls
            .child(muted("留空表示不启用该条件", cx))
            .child(
                Input::new(&self.sum_input)
                    .small()
                    .disabled(self.status.is_active()),
            )
            .child(
                Input::new(&self.score_input)
                    .small()
                    .disabled(self.status.is_active()),
            )
            .child(
                Input::new(&self.pattern_input)
                    .small()
                    .disabled(self.status.is_active()),
            )
            .child(
                Switch::new("early-stop")
                    .small()
                    .label("允许不足五张时命中停止条件")
                    .checked(self.early_stop)
                    .disabled(self.status.is_active())
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.action(Action::EarlyStop, window, cx)
                    })),
            )
            .when_some(self.battle_conditions(cx).err(), |el, error| {
                el.child(muted(error, cx))
            });
        let mut history = div()
            .id("hands")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_2()
            .gap_2();
        for (i, hand) in self.hands.iter().enumerate().rev() {
            history = history.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(heading(
                                format!(
                                    "#{}  {}",
                                    i + 1,
                                    hand.hand.map(hand_name).unwrap_or("未构成牌型")
                                ),
                                cx,
                            ))
                            .child(div().text_color(cx.theme().primary).font_semibold().child(
                                format!(
                                    "{} 分{}",
                                    hand.score,
                                    if hand.lucky { "  好运组合 ×2" } else { "" }
                                ),
                            )),
                    )
                    .child(div().flex().gap_2().children(hand.cards.iter().map(|card| {
                        div()
                            .w(px(56.))
                            .h_20()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_lg()
                            .child(format!(
                                "{}{}",
                                card.rank,
                                match card.suit {
                                    gamble_battle::Suit::Bing => "饼",
                                    gamble_battle::Suit::Tiao => "条",
                                    gamble_battle::Suit::Wan => "万",
                                }
                            ))
                    })))
                    .child(muted(
                        format!("第 {} 轮，耗时 {:.1} 秒", hand.round, hand.elapsed_seconds),
                        cx,
                    ))
                    .when_some(archive::decode_image(&hand.image), |el, image| {
                        el.child(
                            img(image)
                                .w_full()
                                .h(px(160.))
                                .object_fit(ObjectFit::Contain),
                        )
                    }),
            );
        }
        if self.hands.is_empty() {
            history = history.child(empty("暂无命中记录", cx));
        }
        div()
            .flex()
            .size_full()
            .gap_2()
            .child(
                self.panel(cx)
                    .w(px(320.))
                    .flex_shrink_0()
                    .child(
                        div()
                            .flex()
                            .h(px(56.))
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(heading("百趣演武", cx))
                            .child(
                                self.button(
                                    "battle",
                                    if self.status.is_active() {
                                        "停止"
                                    } else {
                                        "开始"
                                    },
                                    Action::Battle,
                                    true,
                                    cx,
                                )
                                .disabled(
                                    self.backend.is_none()
                                        || (!self.status.is_active()
                                            && self.battle_conditions(cx).is_err()),
                                ),
                            ),
                    )
                    .child(controls),
            )
            .child(
                self.panel(cx)
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .h(px(56.))
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(heading(format!("命中记录  {}", self.hands.len()), cx))
                            .child(
                                self.button("clear-hands", "清空", Action::ClearHands, false, cx)
                                    .disabled(self.status.is_active()),
                            ),
                    )
                    .child(history)
                    .when_some(self.progress.clone(), |el, p| {
                        el.child(footer(
                            format!("第 {} 轮，耗时 {:.1} 秒", p.round, p.elapsed_seconds),
                            cx,
                        ))
                    }),
            )
    }

    pub(super) fn logs_page(&self, cx: &Context<Self>) -> impl IntoElement {
        self.panel(cx)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(56.))
                    .px_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(self.button("open-logs", "打开日志目录", Action::OpenLogs, false, cx))
                    .child(
                        Button::new("log-level")
                            .small()
                            .label(["全部等级", "INFO", "WARN", "ERROR"][self.log_filter])
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.log_filter = (view.log_filter + 1) % 4;
                                cx.notify();
                            })),
                    )
                    .child(self.button("clear-logs", "清空日志", Action::ClearLogs, false, cx)),
            )
            .child(
                div()
                    .id("logs")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_3()
                    .gap_2()
                    .children(
                        self.logs
                            .iter()
                            .filter(|e| match self.log_filter {
                                1 => matches!(
                                    e.level,
                                    oea_logger::LogLevel::Info
                                        | oea_logger::LogLevel::Warn
                                        | oea_logger::LogLevel::Error
                                ),
                                2 => matches!(
                                    e.level,
                                    oea_logger::LogLevel::Warn | oea_logger::LogLevel::Error
                                ),
                                3 => matches!(e.level, oea_logger::LogLevel::Error),
                                _ => true,
                            })
                            .map(|e| {
                                div()
                                    .text_sm()
                                    .child(format!("{}  {:?}  {}", e.time, e.level, e.message))
                            }),
                    ),
            )
    }

    pub(super) fn monitor_page(&self, cx: &Context<Self>) -> impl IntoElement {
        self.panel(cx)
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(56.))
                    .gap_3()
                    .px_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        self.choice_menu(
                            "fps",
                            format!("{} FPS", [0.5, 1., 2., 5., 10., 30.][self.frame]),
                            [0.5, 1., 2., 5., 10., 30.]
                                .iter()
                                .enumerate()
                                .map(|(i, fps)| {
                                    (
                                        format!("{fps} FPS").into(),
                                        Action::FrameRate(i),
                                        self.frame == i,
                                    )
                                })
                                .collect(),
                            cx,
                        ),
                    )
                    .child(self.button(
                        "monitor",
                        if self.monitor {
                            "停止监控"
                        } else {
                            "开始监控"
                        },
                        Action::Monitor,
                        self.monitor,
                        cx,
                    ))
                    .child(muted("1280 × 720", cx)),
            )
            .when_some(self.monitor_error.clone(), |el, error| {
                el.child(div().p_3().text_color(rgb(0xef4444)).child(error))
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .p_3()
                    .when_some(self.monitor_image.clone(), |el, image| {
                        el.child(img(image).size_full().object_fit(ObjectFit::Contain))
                    })
                    .when(self.monitor_image.is_none(), |el| {
                        el.child(empty("等待游戏窗口截图", cx))
                    }),
            )
    }

    pub(super) fn guide_page(&self, cx: &Context<Self>) -> impl IntoElement {
        self.panel(cx).child(
            div()
                .id("guide")
                .flex()
                .flex_col()
                .gap_4()
                .p_6()
                .overflow_y_scroll()
                .child(heading("档案扫描前的准备", cx))
                .child(label(
                    "请先启动终末地并进入游戏。扫描会控制游戏窗口，遍历档案库并识别收集状态。",
                    cx,
                ))
                .child(label(
                    "扫描期间请不要操作游戏，保持窗口可见并关闭 HDR。游戏界面需使用简体中文。",
                    cx,
                ))
                .child(label(
                    "按单引号开始或停止档案扫描。百趣演武使用分号热键。Alt+Delete 退出应用。",
                    cx,
                ))
                .child(label(
                    "人工编辑匹配会保留原始识别文本与后端匹配建议。导出前请检查待确认记录。",
                    cx,
                ))
                .child(
                    self.button(
                        "confirm-guide",
                        "确认并开始扫描",
                        Action::ConfirmGuide,
                        true,
                        cx,
                    )
                    .disabled(self.backend.is_none()),
                )
                .child(self.button("dismiss-guide", "返回", Action::Guide, false, cx)),
        )
    }

    pub(super) fn settings_page(&self, cx: &Context<Self>) -> impl IntoElement {
        let disabled = self.backend.is_none() || self.saving;
        let mut body = div()
            .id("settings-body")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .gap_4()
            .child(heading("语言与外观", cx))
            .child(
                self.button(
                    "language",
                    match self.draft_settings.ui_locale {
                        UiLocale::ZhCn => "简体中文",
                        UiLocale::EnUs => "English",
                    },
                    Action::Language,
                    false,
                    cx,
                )
                .disabled(disabled),
            )
            .child(self.button(
                "theme",
                if cx.theme().is_dark() {
                    "深色"
                } else {
                    "浅色"
                },
                Action::Theme,
                false,
                cx,
            ))
            .child(self.button(
                "accent",
                appearance::ACCENTS[self.appearance.accent % appearance::ACCENTS.len()].0,
                Action::Accent,
                false,
                cx,
            ))
            .child(heading("应用行为", cx));
        for (i, (name, checked)) in [
            (
                "关闭窗口时最小化到托盘",
                self.draft_settings.minimize_to_tray,
            ),
            ("自动下载更新", self.draft_settings.auto_download_updates),
            ("自动安装更新", self.draft_settings.auto_install_updates),
        ]
        .into_iter()
        .enumerate()
        {
            body = body.child(
                Switch::new(("setting", i))
                    .small()
                    .label(label(name, cx))
                    .checked(checked)
                    .disabled(disabled)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.action(Action::ToggleSetting(i), window, cx)
                    })),
            );
        }
        body = body
            .child(muted("音效音量（0–100）", cx))
            .child(Input::new(&self.volume_input).small().disabled(disabled))
            .child(heading("更新", cx))
            .child(
                self.button(
                    "source",
                    format!(
                        "更新源：{}",
                        match self.draft_settings.update_source {
                            settings::UpdateSource::Mirrorchyan => "Mirror酱",
                            settings::UpdateSource::Oem => "OEM",
                            settings::UpdateSource::Github => "GitHub",
                        }
                    ),
                    Action::Source,
                    false,
                    cx,
                )
                .disabled(disabled),
            )
            .child(Input::new(&self.cdk_input).small().disabled(disabled))
            .child(
                self.button(
                    "proxy",
                    format!("代理：{:?}", self.draft_settings.update_proxy_mode),
                    Action::Proxy,
                    false,
                    cx,
                )
                .disabled(disabled),
            )
            .child(Input::new(&self.proxy_input).small().disabled(disabled))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        self.button("check-update", "检查更新", Action::CheckUpdate, false, cx)
                            .disabled(disabled || self.update_task.is_some()),
                    )
                    .child(
                        self.button("download-update", "下载", Action::DownloadUpdate, false, cx)
                            .disabled(disabled || self.update_task.is_some()),
                    )
                    .child(self.button(
                        "cancel-download",
                        "取消下载",
                        Action::CancelDownload,
                        false,
                        cx,
                    ))
                    .child(
                        self.button("install-update", "安装", Action::InstallUpdate, false, cx)
                            .disabled(self.status.is_active() || self.update_task.is_some()),
                    ),
            )
            .when_some(self.update_message.clone(), |el, message| el.child(message))
            .when_some(self.download_progress.clone(), |el, p| {
                el.child(format!(
                    "{:.1}%  {} / {} 字节，{} 字节/秒",
                    p.progress, p.downloaded_size, p.total_size, p.speed
                ))
            })
            .child(heading("开发者选项", cx))
            .child(
                self.button(
                    "worker",
                    if self.simulation {
                        "执行方式：模拟"
                    } else {
                        "执行方式：真实游戏"
                    },
                    Action::WorkerType,
                    false,
                    cx,
                )
                .disabled(self.status.is_active()),
            )
            .child(self.button(
                "restart-admin",
                "以管理员权限重启",
                Action::RestartAdmin,
                false,
                cx,
            ))
            .child(
                self.button(
                    "developer-update",
                    "安装本地更新包",
                    Action::DeveloperInstall,
                    false,
                    cx,
                )
                .disabled(self.status.is_active() || self.update_task.is_some()),
            )
            .child(heading("关于 OEA", cx))
            .child(format!("Open Endfield Assistant {}", version()))
            .child(muted("GPUI 原生桌面", cx));
        self.panel(cx)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(56.))
                    .px_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(heading("设置", cx))
                    .child(
                        self.button(
                            "save-settings",
                            if self.saving {
                                "保存中…"
                            } else {
                                "保存设置"
                            },
                            Action::SaveSettings,
                            true,
                            cx,
                        )
                        .disabled(disabled),
                    ),
            )
            .child(body)
    }

    pub(super) fn help_page(&self, cx: &Context<Self>) -> impl IntoElement {
        self.panel(cx).child(div().id("help").flex().flex_col().gap_4().p_4().overflow_y_scroll()
            .child(heading("Open Endfield Assistant", cx))
            .child(label("档案扫描会连接真实游戏窗口，识别档案并生成扫描记录。点击记录可以预览截图，编辑匹配时仅选择同分类档案。", cx))
            .child(label("百趣演武根据牌型、战力点之和、分数或战力点子串停止。条件满足任意一项即可停止，好运组合由后端按北京时间凌晨 4 点更新。", cx))
            .child(label("单引号：开始或停止档案扫描。分号：使用当前条件开始或停止百趣演武。Alt+Delete：退出。任务热键仅在 OEA 或游戏窗口位于前台时生效。", cx))
            .child(label("自动化与游戏监控支持 Windows。macOS 可开发原生界面与运行模拟工作者。", cx))
            .child(self.button("scan-guide","查看扫描指引",Action::Guide,false,cx))
            .child(Button::new("github").small().label("GitHub").on_click(|_,_,_|{let _=open::that_detached("https://github.com/Logical-Byte/open-endfield-assistant");})))
    }

    pub(super) fn save_settings(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(backend) = &self.backend else {
            return;
        };
        let mut settings = self.draft_settings.clone();
        let Ok(volume) = self.volume_input.read(cx).value().parse::<f32>() else {
            self.error = Some("音量必须是 0 到 100 的数字".into());
            return;
        };
        if !(0. ..=100.).contains(&volume) {
            self.error = Some("音量必须在 0 到 100 之间".into());
            return;
        }
        settings.sound_volume = volume / 100.;
        settings.update_proxy_url = self.proxy_input.read(cx).value().to_string();
        let cdk = self.cdk_input.read(cx).value().to_string();
        let controller = Arc::clone(&backend.controller);
        self.saving = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if !cdk.is_empty() {
                        use base64::Engine;
                        settings.mirrorchyan_cdk_encrypted =
                            base64::engine::general_purpose::STANDARD
                                .encode(crate::platform::data_protection::encrypt(cdk.as_bytes())?);
                    }
                    controller.settings_store().save(settings.clone())?;
                    Ok::<_, anyhow::Error>(settings)
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.saving = false;
                match result {
                    Ok(settings) => {
                        cx.set_global(i18n::Language(settings.ui_locale));
                        view.draft_settings = settings;
                        view.notice = Some("设置已保存".into());
                    }
                    Err(error) => view.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn update(&mut self, action: Action, cx: &mut Context<Self>) {
        if self.update_task.is_some() {
            return;
        }
        let Some(backend) = &self.backend else {
            return;
        };
        let manager = Arc::clone(&backend.updates);
        let controller = Arc::clone(&backend.controller);
        let (progress_tx, progress_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);
        let (stage_tx, stage_rx) = mpsc::channel();
        self.stage_rx = Some(stage_rx);
        self.update_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    let on_stage = move |stage| {
                        let _ = stage_tx.send(stage);
                    };
                    let result = match action {
                        Action::CheckUpdate => NativeUpdateResult::Checked(runtime.block_on(
                            update::commands::check_update(&manager, &controller, &version()),
                        )?),
                        Action::DownloadUpdate => NativeUpdateResult::Downloaded(
                            runtime.block_on(update::commands::download_update(
                                &manager,
                                &controller,
                                &version(),
                                Arc::new(move |progress| {
                                    progress_tx
                                        .send(progress)
                                        .map_err(|error| error.to_string())
                                }),
                            ))?,
                        ),
                        Action::InstallUpdate => {
                            update::install::install_update(
                                &manager,
                                controller.settings_snapshot().ui_locale,
                                on_stage,
                            )?;
                            NativeUpdateResult::Installed(true)
                        }
                        Action::DeveloperInstall => NativeUpdateResult::Installed(
                            update::install::extra::developer_install_update(
                                &manager,
                                controller.settings_snapshot().ui_locale,
                                on_stage,
                            )?,
                        ),
                        _ => unreachable!(),
                    };
                    Ok::<_, anyhow::Error>(result)
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.update_task = None;
                match result {
                    Ok(result) => {
                        let mut auto_download = false;
                        let message = match result {
                            NativeUpdateResult::Checked(
                                update::commands::UpdateAvailability::Available { update },
                            ) => {
                                auto_download = view.backend.as_ref().is_some_and(|backend| {
                                    backend.controller.settings_snapshot().auto_download_updates
                                });
                                format!("发现更新 {}\n{}", update.version_name, update.release_note)
                            }
                            NativeUpdateResult::Checked(
                                update::commands::UpdateAvailability::UpToDate,
                            ) => "当前已是最新版本".into(),
                            NativeUpdateResult::Downloaded(
                                update::commands::DownloadOutcome::Completed { .. },
                            ) => "下载完成".into(),
                            NativeUpdateResult::Downloaded(
                                update::commands::DownloadOutcome::Cancelled,
                            )
                            | NativeUpdateResult::Installed(false) => "已取消".into(),
                            NativeUpdateResult::Installed(true) => {
                                if let Some(backend) = &view.backend {
                                    backend.controller.request_shutdown();
                                }
                                cx.quit();
                                "更新已交接".into()
                            }
                        };
                        view.update_message = Some(message);
                        if auto_download {
                            view.update(Action::DownloadUpdate, cx);
                        }
                    }
                    Err(error) => view.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        }));
    }
}

fn version() -> String {
    env!("OEA_VERSION").to_owned()
}

enum NativeUpdateResult {
    Checked(update::commands::UpdateAvailability),
    Downloaded(update::commands::DownloadOutcome),
    Installed(bool),
}
