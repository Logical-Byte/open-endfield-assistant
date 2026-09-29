//! 自动化模块向观察者发布的事件。
//!
//! 工作流通过 [`EventSink`] 发布已经发生的领域事实，不依赖 Tauri 或其他传输机制。

use super::{
    archive_scan::ScanResult,
    runtime::{RunFinished, Status},
};

/// 自动化模块能够向外发布的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    StatusChanged(Status),
    RunFinished(RunFinished),
    ArchiveScanResult(ScanResult),
}

/// 自动化事件的观察出口。
pub(crate) trait EventSink: Send + Sync + 'static {
    fn publish(&self, event: Event);
}

#[cfg(test)]
pub(crate) mod testing {
    use std::{
        sync::{Condvar, Mutex},
        time::Duration,
    };

    use super::{Event, EventSink};

    /// 测试使用的事件记录器，不依赖 Tauri 运行时。
    #[derive(Default)]
    pub(crate) struct RecordingEventSink {
        events: Mutex<Vec<Event>>,
        changed: Condvar,
    }

    impl RecordingEventSink {
        pub(crate) fn wait_for_events(&self, count: usize) -> Vec<Event> {
            let events = self.events.lock().unwrap();
            let (events, _) = self
                .changed
                .wait_timeout_while(events, Duration::from_secs(1), |events| {
                    events.len() < count
                })
                .unwrap();
            events.clone()
        }
    }

    impl EventSink for RecordingEventSink {
        fn publish(&self, event: Event) {
            self.events.lock().unwrap().push(event);
            self.changed.notify_all();
        }
    }
}
