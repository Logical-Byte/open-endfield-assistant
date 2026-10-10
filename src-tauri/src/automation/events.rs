//! 自动化模块向观察者发布的事件。
//!
//! 工作流通过 [`EventSink`] 发布已经发生的领域事实，隐藏底层的 Tauri 或其他传输机制。

use super::{archive_scan::ScannedItem, runtime::Status};

/// 自动化模块能够向外发布的事件。
#[derive(Debug, Clone)]
pub(crate) enum Event {
    StatusChanged(Status),
    ArchiveItemScanned(ScannedItem),
}

/// 自动化事件 [`Event`] 的观察出口。
///
/// 调用者不会观察到发布是否成功。具体实现应自行处理失败、保持同一发送路径上的事件顺序，
/// 并快速接受事件，避免在 `Worker` thread 上执行较重的序列化或外部派发。
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
