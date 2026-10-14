use std::sync::{Arc, Mutex};

use crate::automation::{self, Event, EventSink};

use super::model::{Outcome, Phase, Progress, Report, ScanRecord};

/// 当前/最近一次基质扫描报告的后端事实来源，前端可随时取快照。
#[derive(Default)]
pub(crate) struct ReportStore(Mutex<StoredReport>);

#[derive(Default)]
struct StoredReport {
    next_run_id: u32,
    report: Option<Report>,
}

impl ReportStore {
    pub(crate) fn snapshot(&self) -> Option<Report> {
        self.0.lock().unwrap().report.clone()
    }
}

pub(super) struct Reporter {
    store: Arc<ReportStore>,
    events: Arc<dyn EventSink>,
    run_id: u32,
}

impl Reporter {
    pub(super) fn begin(
        store: Arc<ReportStore>,
        events: Arc<dyn EventSink>,
        worker_type: automation::WorkerType,
    ) -> Self {
        let progress = {
            let mut stored = store.0.lock().unwrap();
            stored.next_run_id += 1;
            let progress = Progress {
                run_id: stored.next_run_id,
                worker_type,
                phase: Phase::Preparing,
                scanned_count: 0,
                incomplete_count: 0,
                outcome: None,
            };
            stored.report = Some(Report {
                progress: progress.clone(),
                records: Vec::new(),
            });
            progress
        };
        let run_id = progress.run_id;
        events.publish(Event::EssenceScanProgress(progress));
        Self {
            store,
            events,
            run_id,
        }
    }

    pub(super) fn phase(&self, phase: Phase) {
        let progress = {
            let mut stored = self.store.0.lock().unwrap();
            let report = stored.report.as_mut().expect("扫描已开始");
            report.progress.phase = phase;
            report.progress.clone()
        };
        self.events.publish(Event::EssenceScanProgress(progress));
    }

    pub(super) fn record(&self, mut record: ScanRecord) {
        let (record, progress) = {
            let mut stored = self.store.0.lock().unwrap();
            let report = stored.report.as_mut().expect("扫描已开始");
            record.run_id = self.run_id;
            report.progress.scanned_count += 1;
            record.record_id = report.progress.scanned_count;
            if !record.issues.is_empty() {
                report.progress.incomplete_count += 1;
            }
            report.records.push(record.clone());
            (record, report.progress.clone())
        };
        self.events.publish(Event::EssenceItemScanned(record));
        self.events.publish(Event::EssenceScanProgress(progress));
    }

    pub(super) fn finish(&self, result: &Result<(), automation::WorkerError>) {
        let progress = {
            let mut stored = self.store.0.lock().unwrap();
            let report = stored.report.as_mut().expect("扫描已开始");
            report.progress.phase = Phase::Finished;
            report.progress.outcome = Some(match result {
                Ok(()) => Outcome::Completed,
                Err(error) if error.is_stopped_by_user() => Outcome::Stopped,
                Err(error) => Outcome::Failed {
                    error: error.clone(),
                },
            });
            report.progress.clone()
        };
        self.events.publish(Event::EssenceScanProgress(progress));
    }
}
