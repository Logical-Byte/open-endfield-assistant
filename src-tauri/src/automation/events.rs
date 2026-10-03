//! 自动化模块向观察者发布的事件。
//!
//! 工作流通过 [`EventSink`] 发布已经发生的领域事实，隐藏底层的 Tauri 或其他传输机制。

use super::{archive_scan, essence_scan, runtime::Status};

/// 自动化模块能够向外发布的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    ArchiveItemScanned(archive_scan::ScannedItem),
    EssenceItemScanned(essence_scan::ScannedItem),
}

/// 自动化事件 [`Event`] 的观察出口。
///
/// 调用者不会观察到发布是否成功。具体实现应自行处理失败、保持同一发送路径上的事件顺序，
/// 并快速接受事件，避免在 `Worker` thread 上执行较重的序列化或外部派发。
pub(crate) trait EventSink: Send + Sync + 'static {
    fn publish(&self, event: Event);
}

/// 运行时向应用层发布状态，以及已经归属于具体运行的领域事件。
pub(crate) trait RuntimeEventSink: Send + Sync + 'static {
    fn publish_status(&self, status: Status);
    fn publish_run_event(&self, run_id: u32, event: Event);
}

/// 前端事件的运行标识与领域载荷，领域类型本身无需持有运行时信息。
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunEvent<T> {
    pub run_id: u32,
    pub payload: T,
}

#[cfg(test)]
pub(crate) mod testing {
    use std::{
        sync::{Condvar, Mutex},
        time::Duration,
    };

    use super::{Event, RuntimeEventSink, Status};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum RecordedEvent {
        Status(Status),
        Run { run_id: u32, event: Event },
    }

    /// 测试使用的事件记录器，不依赖 Tauri 运行时。
    #[derive(Default)]
    pub(crate) struct RecordingEventSink {
        events: Mutex<Vec<RecordedEvent>>,
        changed: Condvar,
    }

    impl RecordingEventSink {
        pub(crate) fn wait_for_events(&self, count: usize) -> Vec<RecordedEvent> {
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

    impl RecordingEventSink {
        fn record(&self, event: RecordedEvent) {
            self.events.lock().unwrap().push(event);
            self.changed.notify_all();
        }
    }

    impl RuntimeEventSink for RecordingEventSink {
        fn publish_status(&self, status: Status) {
            self.record(RecordedEvent::Status(status));
        }

        fn publish_run_event(&self, run_id: u32, event: Event) {
            self.record(RecordedEvent::Run { run_id, event });
        }
    }
}
