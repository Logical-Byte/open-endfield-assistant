//! 真实游戏工作者与仅产生演示观察记录的模拟方式。

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use tracing::{info, warn};

use crate::{
    automation::{
        self, EventSink, StopToken, capabilities,
        runtime::{self, WorkerExit},
        session::Session,
        stats::counts::{Capture, CaptureSummary},
    },
    data::{AppData, essence},
    platform, vision,
};

use super::{
    ReportStore,
    model::{Phase, Rarity, RecognitionIssue, ScanRecord, TermObservation},
    reporting::Reporter,
    workflow,
};

pub(crate) struct Worker {
    worker_type: automation::WorkerType,
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    data: Arc<AppData>,
    reports: Arc<ReportStore>,
}

impl Worker {
    pub(crate) fn new(
        worker_type: automation::WorkerType,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
        data: Arc<AppData>,
        reports: Arc<ReportStore>,
    ) -> Self {
        Self {
            worker_type,
            ocr,
            data,
            reports,
        }
    }

    fn execute(
        &self,
        stop: StopToken,
        reporter: &Reporter,
        capture: &mut Option<CaptureSummary>,
    ) -> Result<(), automation::WorkerError> {
        match self.worker_type {
            automation::WorkerType::Simulation => {
                simulate(&stop, self.data.essences().catalog(), reporter)
            }
            automation::WorkerType::Production => {
                let mut session = Session::connect(&self.ocr, stop)?;
                session.check_stop()?;
                if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
                    warn!(error = %error, "无法将游戏窗口置于前台，继续尝试基质扫描");
                }
                let mut io = Capture::new(&mut session);
                let result = workflow::scan(&mut io, self.data.essences().catalog(), reporter);
                *capture = Some(io.finish());
                result
            }
        }
    }
}

impl runtime::Worker for Worker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let reporter = Reporter::begin(Arc::clone(&self.reports), events, self.worker_type);
        info!("开始基质扫描，结果范围为游戏当前显示的基质列表");
        let mut capture = None;
        let result = self.execute(stop, &reporter, &mut capture);
        reporter.finish(&result);
        WorkerExit { result, capture }
    }
}

fn simulate(
    stop: &StopToken,
    catalog: &essence::Catalog,
    reporter: &Reporter,
) -> Result<(), automation::WorkerError> {
    reporter.phase(Phase::Scanning);
    // 模拟传输与展示，不读取或操作游戏。前两枚刻意相同，另包含未知字段。
    for index in 0..12 {
        if automation::is_stop_requested(stop) {
            return Err(capabilities::Error::StoppedByUser.into());
        }
        let terms = [
            essence::TermKind::Attribute,
            essence::TermKind::Secondary,
            essence::TermKind::Skill,
        ]
        .map(|kind| {
            let term = catalog
                .terms
                .iter()
                .find(|term| term.kind == kind)
                .expect("目录包含三类词条");
            TermObservation {
                term_id: Some(term.id.clone()),
                kind: Some(kind),
                level: Some(3),
            }
        });
        let mut record = ScanRecord {
            run_id: 0,
            record_id: 0,
            row: index / 9 + 1,
            column: index % 9 + 1,
            terms,
            rarity: if index < 6 {
                Rarity::Five
            } else {
                Rarity::Four
            },
            locked: Some(index >= 2 && index % 2 == 0),
            abandoned: Some(index % 3 == 2),
            issues: Vec::new(),
            image: String::new(),
        };
        if index == 4 {
            record.terms[2].term_id = None;
            record.terms[2].kind = None;
            record.terms[2].level = None;
            record.locked = None;
            record.issues = vec![
                RecognitionIssue::Term { position: 3 },
                RecognitionIssue::Level { position: 3 },
                RecognitionIssue::Locked,
            ];
        }
        reporter.record(record);
        thread::sleep(Duration::from_millis(150));
    }
    Ok(())
}
