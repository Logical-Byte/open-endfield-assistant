use super::*;
use crate::{data, navigation, platform};
use std::sync::Mutex;

pub(super) struct Backend {
    pub controller: Arc<Controller>,
    pub updates: Arc<update::UpdateManager>,
}
struct EventSink(mpsc::Sender<automation::Event>);
impl automation::EventSink for EventSink {
    fn publish(&self, event: automation::Event) {
        let _ = self.0.send(event);
    }
}

impl Backend {
    pub fn load(paths: AppPaths, events: mpsc::Sender<automation::Event>) -> anyhow::Result<Self> {
        if update::allow_variant_update() {
            update::install::initialize_at_startup(&paths)?;
        }
        platform::window::set_thread_dpi_awareness_context();
        let app_data = data::AppData::load(&paths)?;
        let ocr = oea_vision::ocr::OcrEngine::new(&paths.models_dir(), Default::default())?;
        let runtime = automation::Runtime::new(Arc::new(EventSink(events)));
        Ok(Self {
            controller: Arc::new(Controller::new(
                Arc::new(settings::SettingsStore::at(paths.oea_settings_file())),
                Arc::new(Mutex::new(ocr)),
                Arc::new(navigation::Navigator::new()),
                Arc::new(runtime),
                app_data,
            )),
            updates: Arc::new(update::UpdateManager::default()),
        })
    }
}

/// 持有与桌面生命周期绑定的后台资源。键盘事件由 GPUI 的非阻塞事件泵消费。
pub(super) struct BackgroundThreads {
    pub logs: mpsc::Receiver<oea_logger::LogEntry>,
    pub events: mpsc::Receiver<automation::Event>,
    pub event_sender: mpsc::Sender<automation::Event>,
    pub keyboard: mpsc::Receiver<platform::hotkey::KeyEvent>,
    hook: Option<platform::hotkey::KeyboardHookGuard>,
    _logger: oea_logger::LogGuard,
    pub tray: Option<tray_icon::TrayIcon>,
    pub tray_attempted: bool,
    pub tray_show: tray_icon::menu::MenuId,
    pub tray_scan: tray_icon::menu::MenuId,
    pub tray_quit: tray_icon::menu::MenuId,
}
impl BackgroundThreads {
    pub fn new(paths: &AppPaths) -> anyhow::Result<Self> {
        let (logger, logs) = oea_logger::init(&paths.logs_dir());
        let (event_sender, events) = mpsc::channel();
        let (keyboard, hook) = platform::hotkey::listen()?;
        // 托盘必须在 GPUI 主线程的事件循环启动后创建。
        Ok(Self {
            logs,
            events,
            event_sender,
            keyboard,
            hook: Some(hook),
            _logger: logger,
            tray: None,
            tray_attempted: false,
            tray_show: tray_icon::menu::MenuId::new("show"),
            tray_scan: tray_icon::menu::MenuId::new("scan"),
            tray_quit: tray_icon::menu::MenuId::new("quit"),
        })
    }
    pub fn initialize_tray(&mut self) -> anyhow::Result<()> {
        let menu = tray_icon::menu::Menu::new();
        menu.append_items(&[
            &tray_icon::menu::MenuItem::with_id(self.tray_show.clone(), "打开 OEA", true, None),
            &tray_icon::menu::MenuItem::with_id(
                self.tray_scan.clone(),
                "开始 / 停止档案扫描",
                true,
                None,
            ),
            &tray_icon::menu::MenuItem::with_id(self.tray_quit.clone(), "退出", true, None),
        ])?;
        let image =
            image::load_from_memory(include_bytes!("../../../../resources/icons/icon.png"))?
                .to_rgba8();
        let (w, h) = image.dimensions();
        self.tray = Some(
            tray_icon::TrayIconBuilder::new()
                .with_tooltip("OEA")
                .with_icon(tray_icon::Icon::from_rgba(image.into_raw(), w, h)?)
                .with_menu(Box::new(menu))
                .build()?,
        );
        Ok(())
    }
}
impl Drop for BackgroundThreads {
    fn drop(&mut self) {
        if let Some(hook) = self.hook.take()
            && let Err(error) = hook.shutdown()
        {
            tracing::error!("关闭键盘监听失败：{error}");
        }
    }
}

pub(super) fn hotkey_scope(app_active: bool) -> bool {
    app_active
        || oea_window::search(automation::game_environment::window_query())
            .ok()
            .flatten()
            .and_then(|w| w.is_foreground().ok())
            .unwrap_or(false)
}

pub(super) fn capture() -> anyhow::Result<Arc<gpui::Image>> {
    let env = automation::game_environment::connect(automation::game_environment::Requirements {
        restore_if_minimized: false,
        ensure_on_screen: false,
        hdr_disabled: false,
    })?;
    let image = env.window.screenshot()?;
    let resized = image::imageops::resize(&image, 1280, 720, image::imageops::FilterType::Triangle);
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(resized).write_to(&mut png, image::ImageFormat::Png)?;
    Ok(Arc::new(gpui::Image::from_bytes(
        gpui::ImageFormat::Png,
        png.into_inner(),
    )))
}

impl Oea {
    pub(super) fn receive_tray(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if event.id == self.background.tray_show {
                window.activate_window();
                cx.activate(true);
            } else if event.id == self.background.tray_scan {
                self.action(Action::Scan, window, cx);
            } else if event.id == self.background.tray_quit {
                self.quit(cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::path::PathBuf;
    use std::{io::Read, time::Duration};

    /// 穿过真实运行时、领域事件、原生记录模型与 OEM 编码，不创建桌面窗口。
    #[::core::prelude::v1::test]
    fn native_records_consume_real_worker_events_and_export_stable_archive_ids() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = data::AppData::load(&AppPaths::with_root_dir(root)).unwrap();
        let archives = archive::project(data.archives().catalog());
        let (tx, rx) = mpsc::channel();
        let runtime = Arc::new(automation::Runtime::new(Arc::new(EventSink(tx))));
        runtime.start(automation::TaskKind::ArchiveScan, || {
            Box::new(archive_scan::SimulationWorker::new(Arc::new(data)))
        });
        let record = loop {
            match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                automation::Event::ArchiveItemScanned(item) => {
                    break ScanRecord::new(1, item, &archives);
                }
                _ => continue,
            }
        };
        runtime.stop();
        while !matches!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            automation::Event::StatusChanged(automation::Status::Idle { .. })
        ) {}
        assert_eq!(record.observation.ocr_result, "受困者的录音");
        let target = record.target.expect("真实匹配器应命中首条模拟记录");
        assert_eq!(archives[target].title, "受困者的录音");
        assert!(record.image.is_some());
        let url = archive::export_url(&archives, &[record]).unwrap();
        let gzip = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(url.strip_prefix("https://oem.re/i/OEA-0-").unwrap())
            .unwrap();
        let mut json = String::new();
        flate2::read::GzDecoder::new(gzip.as_slice())
            .read_to_string(&mut json)
            .unwrap();
        let upload: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            upload["data"]["prtsAllItems"]["collected"],
            serde_json::json!([archives[target].id])
        );
        assert_eq!(
            upload["data"]["prtsAllItems"]["notCollected"]
                .as_array()
                .unwrap()
                .len(),
            archives.len() - 1
        );
    }
}
