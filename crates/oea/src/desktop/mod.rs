//! GPUI 桌面入口。调用原有控制器与自动化引擎，不经过 IPC 或 WebView。
mod appearance;
mod archive;
mod backend;
mod i18n;
mod pages;
mod ui;

use crate::{
    app_paths::AppPaths,
    automation::{self, archive_scan, gamble_battle},
    controller::Controller,
    data::archive as catalog,
    settings, update,
};
use archive::{Archive, ScanRecord};
use backend::{Backend, BackgroundThreads};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Root, Sizable, StyledExt, Theme, ThemeMode,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};
use i18n::label;
use std::{
    collections::HashSet,
    sync::{Arc, mpsc},
    time::Duration,
};

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Scan,
    Battle,
    Logs,
    Monitor,
    Settings,
    Help,
}

#[derive(Clone, Default, PartialEq, gpui::Action)]
struct Quit;

#[derive(Clone, Copy)]
enum Action {
    Navigate(Page),
    Theme,
    Accent,
    Collapse,
    Scan,
    ClearScans,
    ArchiveFilter(usize),
    ScanFilter(usize),
    Category(usize),
    Expand(usize),
    Edit(u64),
    Assign(u64, Option<usize>),
    Undo,
    Battle,
    ClearHands,
    Hand(usize),
    EarlyStop,
    ClearLogs,
    Monitor,
    FrameRate(usize),
    CopyCollected,
    Export,
    Guide,
    ConfirmGuide,
    Retry,
    CheckUpdate,
    DownloadUpdate,
    CancelDownload,
    InstallUpdate,
    DeveloperInstall,
    OpenLogs,
    RestartAdmin,
    SaveSettings,
    ToggleSetting(usize),
    Source,
    Proxy,
    Language,
    WorkerType,
    MapOnly,
    Preview(u64),
    ClosePreview,
}

struct Oea {
    appearance: appearance::Appearance,
    page: Page,
    collapsed: bool,
    archives: Vec<Archive>,
    archive_search: Entity<InputState>,
    scan_search: Entity<InputState>,
    match_search: Entity<InputState>,
    sum_input: Entity<InputState>,
    score_input: Entity<InputState>,
    pattern_input: Entity<InputState>,
    volume_input: Entity<InputState>,
    proxy_input: Entity<InputState>,
    cdk_input: Entity<InputState>,
    archive_filter: usize,
    scan_filter: usize,
    category: usize,
    map_only: bool,
    expanded: HashSet<usize>,
    records: Vec<ScanRecord>,
    editing: Option<u64>,
    undo: Vec<(u64, Option<Option<catalog::ArchiveId>>)>,
    next_record_id: u64,
    hands: Vec<gamble_battle::MatchedHand>,
    conditions: [bool; 8],
    early_stop: bool,
    progress: Option<gamble_battle::Progress>,
    status: automation::Status,
    logs: Vec<oea_logger::LogEntry>,
    log_filter: usize,
    monitor: bool,
    frame: usize,
    monitor_image: Option<Arc<gpui::Image>>,
    monitor_error: Option<String>,
    monitor_task: Option<Task<()>>,
    screenshot_at: Option<std::time::Instant>,
    backend: Option<Backend>,
    background: BackgroundThreads,
    initialization: Option<Task<()>>,
    error: Option<String>,
    notice: Option<String>,
    guide: bool,
    simulation: bool,
    draft_settings: settings::OeaSettings,
    saving: bool,
    preview: Option<Arc<gpui::Image>>,
    update_message: Option<String>,
    download_progress: Option<update::commands::DownloadProgress>,
    progress_rx: Option<mpsc::Receiver<update::commands::DownloadProgress>>,
    stage_rx: Option<mpsc::Receiver<update::install::InstallStage>>,
    install_stage: Option<update::install::InstallStage>,
    update_checked: bool,
    update_task: Option<Task<()>>,
    _pump: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Oea {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        paths: AppPaths,
        background: BackgroundThreads,
    ) -> Self {
        let mut input = |placeholder: &'static str| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let archive_search = input("搜索档案标题…");
        let scan_search = input("搜索识别标题…");
        let match_search = input("搜索要匹配的档案…");
        let sum_input = input("战力点之和 1–25");
        let score_input = input("最低分数");
        let pattern_input = input("战力点子串，例如 325");
        let volume_input = input("音量 0–100");
        let proxy_input = input("代理 URL");
        let cdk_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Mirror酱 CDK")
                .masked(true)
        });
        let subscriptions = [
            &archive_search,
            &scan_search,
            &match_search,
            &sum_input,
            &score_input,
            &pattern_input,
        ]
        .into_iter()
        .map(|input| cx.observe(input, |_, _, cx| cx.notify()))
        .collect();
        let appearance = appearance::load(&paths);
        appearance::apply(&appearance, Some(window), cx);
        let mut view = Self {
            page: Page::Scan,
            collapsed: appearance.collapsed,
            appearance,
            archives: vec![],
            archive_search,
            scan_search,
            match_search,
            sum_input,
            score_input,
            pattern_input,
            volume_input,
            proxy_input,
            cdk_input,
            archive_filter: 0,
            scan_filter: 0,
            category: 0,
            map_only: false,
            expanded: HashSet::new(),
            records: vec![],
            editing: None,
            undo: vec![],
            next_record_id: 1,
            hands: vec![],
            conditions: [false; 8],
            early_stop: false,
            progress: None,
            status: automation::Status::Idle { last_run: None },
            logs: vec![],
            log_filter: 0,
            monitor: false,
            frame: 1,
            monitor_image: None,
            monitor_error: None,
            monitor_task: None,
            screenshot_at: None,
            backend: None,
            background,
            initialization: None,
            error: None,
            notice: None,
            guide: false,
            simulation: false,
            draft_settings: settings::SettingsStore::at(paths.oea_settings_file()).snapshot(),
            saving: false,
            preview: None,
            update_message: None,
            download_progress: None,
            progress_rx: None,
            stage_rx: None,
            install_stage: None,
            update_checked: false,
            update_task: None,
            _pump: None,
            _subscriptions: subscriptions,
        };
        view.initialize(paths, cx);
        view._pump = Some(cx.spawn_in(window, async |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                if this
                    .update_in(cx, |view, window, cx| view.receive(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        view
    }

    fn initialize(&mut self, paths: AppPaths, cx: &mut Context<Self>) {
        self.error = None;
        let events = self.background.event_sender.clone();
        self.initialization = Some(cx.spawn(async |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { Backend::load(paths, events) })
                .await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(backend) => {
                        view.archives = archive::project(backend.controller.archive_catalog());
                        view.draft_settings = backend.controller.settings_snapshot();
                        cx.set_global(i18n::Language(view.draft_settings.ui_locale));
                        view.backend = Some(backend);
                        tracing::info!("GPUI 桌面初始化完成");
                    }
                    Err(error) => {
                        tracing::error!("桌面初始化失败：{error:#}");
                        view.error = Some(format!("{error:#}"));
                    }
                }
                view.initialization = None;
                cx.notify();
            });
        }));
    }

    fn receive(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.background.tray_attempted {
            self.background.tray_attempted = true;
            if let Err(error) = self.background.initialize_tray() {
                tracing::warn!("创建系统托盘失败：{error}");
            }
        }
        let mut changed = false;
        if let Some(rx) = &self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.download_progress = Some(progress);
                changed = true;
            }
        }
        if let Some(rx) = &self.stage_rx {
            while let Ok(stage) = rx.try_recv() {
                self.install_stage = Some(stage);
                changed = true;
            }
        }
        while let Ok(entry) = self.background.logs.try_recv() {
            self.logs.push(entry);
            changed = true;
        }
        if self.logs.len() > 10000 {
            self.logs.drain(..self.logs.len() - 10000);
        }
        while let Ok(event) = self.background.events.try_recv() {
            changed = true;
            match event {
                automation::Event::StatusChanged(status) => {
                    if matches!(
                        status,
                        automation::Status::Running {
                            task_kind: automation::TaskKind::ArchiveScan
                        }
                    ) {
                        self.records.clear();
                        self.undo.clear();
                        self.editing = None;
                        self.error = None;
                    }
                    if let automation::Status::Idle {
                        last_run: Some(ref last),
                    } = status
                        && let automation::RunOutcome::Failed { error } = &last.outcome
                        && !matches!(error, automation::Error::Worker(worker) if worker.is_stopped_by_user())
                    {
                        self.error = Some(error.to_string());
                    }
                    self.status = status;
                }
                automation::Event::ArchiveItemScanned(item) => {
                    self.records
                        .push(ScanRecord::new(self.next_record_id, item, &self.archives));
                    self.next_record_id += 1;
                }
                automation::Event::GambleBattleMatched(hand) => self.hands.push(hand),
                automation::Event::GambleBattleProgress(progress) => self.progress = Some(progress),
            }
        }
        while let Ok(event) = self.background.keyboard.try_recv() {
            if !event.down {
                continue;
            }
            if event.vk == crate::platform::hotkey::DELETE_KEY
                && event.modifiers == crate::platform::hotkey::ALT_MODIFIER
            {
                self.quit(cx);
                continue;
            }
            if event.modifiers != 0 || !backend::hotkey_scope(window.is_window_active()) {
                continue;
            }
            match event.vk {
                crate::platform::hotkey::OEM_7_KEY => self.action(Action::Scan, window, cx),
                crate::platform::hotkey::OEM_1_KEY => self.action(Action::Battle, window, cx),
                _ => {}
            }
        }
        self.receive_tray(window, cx);
        if self.backend.is_some() && !self.update_checked && update::allow_variant_update() {
            self.update_checked = true;
            self.update(Action::CheckUpdate, cx);
        }
        if self.update_task.is_none()
            && !self.status.is_active()
            && self.backend.as_ref().is_some_and(|backend| {
                backend.controller.settings_snapshot().auto_install_updates
                    && backend.updates.status().pending_update.is_some()
            })
        {
            self.update(Action::InstallUpdate, cx);
        }
        if self.page == Page::Monitor
            && self.monitor
            && self.monitor_task.is_none()
            && self.screenshot_at.is_none_or(|at| {
                at.elapsed()
                    >= Duration::from_secs_f64(1. / [0.5, 1., 2., 5., 10., 30.][self.frame])
            })
        {
            self.capture(cx);
        }
        if changed {
            cx.notify();
        }
    }

    fn action(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(action, Action::Scan | Action::Battle)
            && self
                .backend
                .as_ref()
                .is_some_and(|backend| backend.updates.is_installing())
        {
            return;
        }
        match action {
            Action::Navigate(page) => {
                if page == Page::Settings && self.page != Page::Settings {
                    self.volume_input.update(cx, |input, cx| {
                        input.set_value(
                            format!("{}", self.draft_settings.sound_volume * 100.),
                            window,
                            cx,
                        )
                    });
                    self.proxy_input.update(cx, |input, cx| {
                        input.set_value(self.draft_settings.update_proxy_url.clone(), window, cx)
                    });
                }
                self.page = page;
                if page == Page::Monitor {
                    self.monitor = true;
                }
            }
            Action::Theme => {
                let mode = if cx.theme().is_dark() {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                };
                Theme::change(mode, Some(window), cx);
                self.appearance.dark = Some(cx.theme().is_dark());
                appearance::apply(&self.appearance, None, cx);
                if let Err(error) = appearance::save(&self.appearance) {
                    self.error = Some(error.to_string());
                }
            }
            Action::Accent => {
                self.appearance.accent = (self.appearance.accent + 1) % appearance::ACCENTS.len();
                appearance::apply(&self.appearance, None, cx);
                if let Err(error) = appearance::save(&self.appearance) {
                    self.error = Some(error.to_string());
                }
            }
            Action::Collapse => {
                self.collapsed = !self.collapsed;
                self.appearance.collapsed = self.collapsed;
                if let Err(error) = appearance::save(&self.appearance) {
                    self.error = Some(error.to_string());
                }
            }
            Action::ArchiveFilter(v) => self.archive_filter = v,
            Action::ScanFilter(v) => self.scan_filter = v,
            Action::Category(index) => self.category = index,
            Action::MapOnly => self.map_only = !self.map_only,
            Action::Expand(i) => {
                if !self.expanded.remove(&i) {
                    self.expanded.insert(i);
                }
            }
            Action::Scan => {
                if self.status.is_active() {
                    self.stop();
                } else if self.draft_settings.scan_tips_dismissed_version < 1 {
                    self.guide = true;
                } else {
                    self.start_scan();
                }
            }
            Action::Guide => {
                self.guide = !self.guide;
                self.page = Page::Scan;
            }
            Action::ConfirmGuide => {
                let mut settings = self.draft_settings.clone();
                settings.scan_tips_dismissed_version = 1;
                if let Some(backend) = &self.backend {
                    match backend.controller.settings_store().save(settings.clone()) {
                        Ok(()) => {
                            self.draft_settings = settings;
                            self.guide = false;
                            self.start_scan();
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::ClearScans => {
                if !self.status.is_active() {
                    self.records.clear();
                    self.editing = None;
                    self.undo.clear();
                }
            }
            Action::Edit(i) => {
                self.editing = if self.editing == Some(i) {
                    None
                } else {
                    Some(i)
                };
                self.match_search
                    .update(cx, |input, cx| input.set_value("", window, cx));
            }
            Action::Assign(i, target) => {
                if let Some(record) = self.records.iter_mut().find(|record| record.id == i) {
                    let target = target.map(|i| self.archives[i].id.clone());
                    if target.as_ref().is_none_or(|id| {
                        self.archives.iter().any(|a| {
                            &a.id == id && a.category_id == record.observation.found_in_category
                        })
                    }) {
                        self.undo.push((record.id, record.manual_choice.clone()));
                        record.manual_choice = Some(target);
                        record.refresh(&self.archives);
                        self.editing = None;
                        tracing::info!(record_id = record.id, "已修改档案匹配");
                    }
                }
            }
            Action::Undo => {
                if let Some((id, choice)) = self.undo.pop()
                    && let Some(record) = self.records.iter_mut().find(|r| r.id == id)
                {
                    record.manual_choice = choice;
                    record.refresh(&self.archives);
                }
            }
            Action::Battle => {
                if self.status.is_active() {
                    self.stop();
                } else {
                    match self.battle_conditions(cx) {
                        Ok(conditions) => {
                            if let Some(backend) = &self.backend {
                                backend.controller.start_automation(
                                    automation::StartRequest::GambleBattle {
                                        conditions,
                                        allow_early_stop: self.early_stop,
                                        worker_type: self.worker_type(),
                                    },
                                );
                            }
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
            Action::Hand(i) => {
                if !self.status.is_active() {
                    self.conditions[i] = !self.conditions[i];
                }
            }
            Action::EarlyStop => {
                if !self.status.is_active() {
                    self.early_stop = !self.early_stop;
                }
            }
            Action::ClearHands => {
                if !self.status.is_active() {
                    self.hands.clear();
                }
            }
            Action::ClearLogs => self.logs.clear(),
            Action::Monitor => self.monitor = !self.monitor,
            Action::FrameRate(index) => self.frame = index,
            Action::CopyCollected => {
                let ids: HashSet<_> = self.records.iter().filter_map(|r| r.target).collect();
                cx.write_to_clipboard(ClipboardItem::new_string(
                    self.archives
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| ids.contains(i))
                        .map(|(_, a)| a.title.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ));
                self.notice = Some("已复制收集结果".into());
            }
            Action::Export => match archive::export_url(&self.archives, &self.records) {
                Ok(url) => {
                    if let Err(error) = open::that_detached(url) {
                        self.error = Some(error.to_string());
                    }
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            Action::Retry => {
                if self.initialization.is_none()
                    && let Ok(paths) = AppPaths::new()
                {
                    self.initialize(paths, cx);
                }
            }
            Action::OpenLogs => {
                if let Ok(paths) = AppPaths::new()
                    && let Err(error) = open::that_detached(paths.logs_dir())
                {
                    self.error = Some(error.to_string());
                }
            }
            Action::RestartAdmin => match crate::platform::admin::restart_as_admin() {
                Ok(()) => self.quit(cx),
                Err(error) => self.error = Some(error.to_string()),
            },
            Action::SaveSettings => self.save_settings(window, cx),
            Action::ToggleSetting(i) => match i {
                0 => self.draft_settings.minimize_to_tray = !self.draft_settings.minimize_to_tray,
                1 => {
                    self.draft_settings.auto_download_updates =
                        !self.draft_settings.auto_download_updates
                }
                _ => {
                    self.draft_settings.auto_install_updates =
                        !self.draft_settings.auto_install_updates
                }
            },
            Action::Source => {
                self.draft_settings.update_source = match self.draft_settings.update_source {
                    settings::UpdateSource::Mirrorchyan => settings::UpdateSource::Oem,
                    settings::UpdateSource::Oem => settings::UpdateSource::Github,
                    _ => settings::UpdateSource::Mirrorchyan,
                }
            }
            Action::Proxy => {
                self.draft_settings.update_proxy_mode = match self.draft_settings.update_proxy_mode
                {
                    settings::UpdateProxyMode::None => settings::UpdateProxyMode::System,
                    settings::UpdateProxyMode::System => settings::UpdateProxyMode::Custom,
                    _ => settings::UpdateProxyMode::None,
                }
            }
            Action::Language => {
                self.draft_settings.ui_locale = match self.draft_settings.ui_locale {
                    crate::locale::UiLocale::ZhCn => crate::locale::UiLocale::EnUs,
                    _ => crate::locale::UiLocale::ZhCn,
                }
            }
            Action::WorkerType => {
                if !self.status.is_active() {
                    self.simulation = !self.simulation;
                }
            }
            Action::Preview(i) => {
                self.preview = self
                    .records
                    .iter()
                    .find(|record| record.id == i)
                    .and_then(|r| r.image.as_ref().map(Arc::clone));
            }
            Action::ClosePreview => self.preview = None,
            Action::CheckUpdate
            | Action::DownloadUpdate
            | Action::InstallUpdate
            | Action::DeveloperInstall => self.update(action, cx),
            Action::CancelDownload => {
                if let Some(backend) = &self.backend
                    && let Err(error) = update::commands::cancel_download(&backend.updates)
                {
                    self.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    fn worker_type(&self) -> automation::WorkerType {
        if self.simulation {
            automation::WorkerType::Simulation
        } else {
            automation::WorkerType::Production
        }
    }
    fn start_scan(&self) {
        if let Some(backend) = &self.backend {
            backend
                .controller
                .start_automation(automation::StartRequest::ArchiveScan {
                    worker_type: self.worker_type(),
                });
        }
    }
    fn stop(&self) {
        if let Some(backend) = &self.backend {
            backend.controller.stop_automation();
        }
    }
    fn quit(&self, cx: &mut App) {
        if self
            .backend
            .as_ref()
            .is_some_and(|b| b.updates.is_installing())
        {
            return;
        }
        if let Some(backend) = &self.backend {
            backend.controller.request_shutdown();
        }
        cx.quit();
    }

    fn capture(&mut self, cx: &mut Context<Self>) {
        self.screenshot_at = Some(std::time::Instant::now());
        self.monitor_task = Some(cx.spawn(async |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { backend::capture() })
                .await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(image) => {
                        view.monitor_image = Some(image);
                        view.monitor_error = None;
                    }
                    Err(error) => view.monitor_error = Some(format!("{error:#}")),
                }
                view.monitor_task = None;
                cx.notify();
            });
        }));
    }
}

pub fn run() {
    crate::startup::crash::install_panic_hook();
    crate::platform::admin::elevate_at_startup();
    let paths = AppPaths::new().expect("无法定位应用目录");
    crate::startup::portable::ensure_extracted(&paths);
    crate::platform::window::set_thread_dpi_awareness_context();
    let background = BackgroundThreads::new(&paths).expect("无法初始化后台资源");
    let app = Application::new().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        ui::configure_theme(cx);
        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.), px(600.))),
                titlebar: Some(TitlebarOptions {
                    title: Some(product_name().into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| Oea::new(window, cx, paths, background));
                let quit_view = view.downgrade();
                cx.on_action(move |_: &Quit, cx| {
                    let _ = quit_view.update(cx, |view, cx| view.quit(cx));
                });
                cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
                cx.set_menus(vec![Menu {
                    name: "OEA".into(),
                    items: vec![MenuItem::action("退出 OEA", Quit)],
                }]);
                let weak = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    weak.update(cx, |view, cx| {
                        if view
                            .backend
                            .as_ref()
                            .is_some_and(|b| b.updates.is_installing())
                        {
                            return false;
                        }
                        if view.backend.as_ref().is_some_and(|backend| {
                            backend.controller.settings_snapshot().minimize_to_tray
                        }) && view.background.tray.is_some()
                        {
                            window.minimize_window();
                            return false;
                        }
                        view.quit(cx);
                        true
                    })
                    .unwrap_or(true)
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("无法创建 GPUI 窗口");
        cx.activate(true);
    });
}

fn product_name() -> &'static str {
    match crate::app_variant::CURRENT {
        crate::app_variant::AppVariant::Dev => "OEA dev",
        crate::app_variant::AppVariant::Release => env!("OEA_PRODUCT_NAME"),
    }
}
