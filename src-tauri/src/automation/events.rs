//! 扫描运行时推送给前端的状态与结束事件格式。

use serde::{Deserialize, Serialize};

use crate::automation::stats::counts::{CapabilityCallCounts, CaptureSummary};

/// 推送给前端的应用状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    /// 扫描档案库任务是否正在运行
    pub running: bool,
    /// 扫描档案库任务结束时的失败原因（仅失败时随结束状态推送一次；成功 / 被停止 / 查询状态时为 `None`）
    #[serde(default)]
    pub scan_error: Option<String>,
}

/// 前端可区分的自动化任务。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationTask {
    ArchiveScan,
}

/// 一次自动化运行的对外终态。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationOutcome {
    Completed,
    Stopped,
    Failed,
}

impl AutomationOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// 推送给前端的一次自动化运行结束事件。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunFinished {
    task: AutomationTask,
    outcome: AutomationOutcome,
    capture: Option<CaptureSummaryPayload>,
}

impl AutomationRunFinished {
    pub(crate) fn archive_scan(
        outcome: AutomationOutcome,
        capture: Option<CaptureSummary>,
    ) -> Self {
        Self {
            task: AutomationTask::ArchiveScan,
            outcome,
            capture: capture.map(CaptureSummaryPayload::from),
        }
    }
}

/// [`CaptureSummary`] 在 Tauri 事件中的稳定序列化格式。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureSummaryPayload {
    elapsed_micros: u64,
    calls: CapabilityCallCountsPayload,
}

impl From<CaptureSummary> for CaptureSummaryPayload {
    fn from(summary: CaptureSummary) -> Self {
        Self {
            elapsed_micros: u64::try_from(summary.elapsed.as_micros()).unwrap_or(u64::MAX),
            calls: summary.calls.into(),
        }
    }
}

/// [`CapabilityCallCounts`] 在 Tauri 事件中的稳定序列化格式。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityCallCountsPayload {
    screenshot: u64,
    click: u64,
    press_key: u64,
    move_mouse_to_safe_position: u64,
    find_template: u64,
    recognize_text: u64,
    sleep: u64,
}

impl From<CapabilityCallCounts> for CapabilityCallCountsPayload {
    fn from(counts: CapabilityCallCounts) -> Self {
        Self {
            screenshot: counts.screenshot,
            click: counts.click,
            press_key: counts.press_key,
            move_mouse_to_safe_position: counts.move_mouse_to_safe_position,
            find_template: counts.find_template,
            recognize_text: counts.recognize_text,
            sleep: counts.sleep,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::automation::stats::counts::{CapabilityCallCounts, CaptureSummary};

    use super::{AutomationOutcome, AutomationRunFinished};

    #[test]
    fn automation_run_finished_uses_the_frontend_wire_format() {
        let event = AutomationRunFinished::archive_scan(
            AutomationOutcome::Stopped,
            Some(CaptureSummary {
                elapsed: Duration::from_micros(1_234_567),
                calls: CapabilityCallCounts {
                    screenshot: 2,
                    click: 3,
                    press_key: 5,
                    move_mouse_to_safe_position: 7,
                    find_template: 11,
                    recognize_text: 13,
                    sleep: 17,
                },
            }),
        );

        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "task": "archiveScan",
                "outcome": "stopped",
                "capture": {
                    "elapsedMicros": 1_234_567,
                    "calls": {
                        "screenshot": 2,
                        "click": 3,
                        "pressKey": 5,
                        "moveMouseToSafePosition": 7,
                        "findTemplate": 11,
                        "recognizeText": 13,
                        "sleep": 17,
                    },
                },
            })
        );
    }

    #[test]
    fn automation_run_finished_has_no_capture_when_the_interval_never_started() {
        let event = AutomationRunFinished::archive_scan(AutomationOutcome::Failed, None);

        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "task": "archiveScan",
                "outcome": "failed",
                "capture": null,
            })
        );
    }
}
