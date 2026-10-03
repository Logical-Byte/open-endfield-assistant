//! 固定约 20 秒的基质样例序列，用真实规则验证前端展示与停止收尾。

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crate::{
    automation::{
        Event, EventSink, StopToken, is_stop_requested,
        runtime::{FinishReason, Worker, WorkerExit},
    },
    essence,
};

use super::ScannedItem;

pub(crate) struct SimulatedEssenceScanWorker {
    settings: essence::ScanSettings,
}

const SAMPLE_COUNT: u32 = 54;
const INTERVAL: Duration = Duration::from_millis(370);

impl SimulatedEssenceScanWorker {
    pub(crate) fn new(settings: essence::ScanSettings) -> Self {
        Self { settings }
    }

    fn scan(&self, stop: &StopToken, events: &dyn EventSink, interval: Duration) -> FinishReason {
        let catalog = essence::Catalog::bundled();
        if let Err(error) = self.settings.validate(catalog) {
            return FinishReason::Failed(format!("基质扫描设置无效: {error:#}"));
        }
        for index in 0..SAMPLE_COUNT {
            if !wait_for_sample(stop, interval) {
                return FinishReason::Stopped;
            }
            let essence = sample(index);
            let evaluation = essence::evaluate(&essence, &self.settings, catalog);
            if is_stop_requested(stop) {
                return FinishReason::Stopped;
            }
            events.publish(Event::EssenceItemScanned(ScannedItem {
                sequence: index + 1,
                page: index / 45 + 1,
                row: index / 9 + 1,
                column: index % 9 + 1,
                essence,
                evaluation,
                image: None,
            }));
        }
        FinishReason::Completed
    }
}

impl Worker for SimulatedEssenceScanWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let reason = self.scan(&stop, events.as_ref(), INTERVAL);
        if matches!(reason, FinishReason::Stopped) {
            // 给前端留出可观察的停止收尾阶段，期间不再产出结果。
            thread::sleep(Duration::from_millis(500));
        }
        WorkerExit::without_capture(reason)
    }
}

fn wait_for_sample(stop: &StopToken, interval: Duration) -> bool {
    let until = Instant::now() + interval;
    while Instant::now() < until {
        if is_stop_requested(stop) {
            return false;
        }
        thread::sleep(
            until
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
    }
    !is_stop_requested(stop)
}

fn sample(index: u32) -> essence::Essence {
    let mut item = essence::Essence {
        // “典范”的完整组合。评估仍使用当前用户设置与 Catalog。
        stats: [
            "gat_passive_attr_main",
            "gat_passive_attr_atk",
            "gst_passive_tactic",
        ]
        .map(|id| Some(id.into())),
        levels: [Some(1); 3],
        rarity: essence::Rarity::Five,
        locked: Some(false),
        abandoned: Some(false),
    };
    match index % 7 {
        1 | 2 | 3 | 5 => {
            // 无现成武器匹配的组合，让保护标记、高等级和自定义规则的效果可见。
            item.stats = [
                "gat_passive_attr_agi",
                "gat_passive_attr_heal",
                "gst_passive_tactic",
            ]
            .map(|id| Some(id.into()));
            match index % 7 {
                2 => item.locked = Some(true),
                3 => item.abandoned = Some(true),
                5 => item.levels[0] = Some(6),
                _ => {}
            }
        }
        4 => item.rarity = essence::Rarity::Four,
        6 => {
            item.stats[1] = None;
            item.levels[1] = None;
        }
        _ => {}
    }
    item
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::automation::{new_stop_token, request_stop};

    use super::*;

    #[derive(Default)]
    struct Samples {
        items: Mutex<Vec<ScannedItem>>,
        stop_after: Option<(usize, StopToken)>,
    }

    impl EventSink for Samples {
        fn publish(&self, event: Event) {
            let Event::EssenceItemScanned(item) = event else {
                panic!("unexpected simulation event");
            };
            let mut items = self.items.lock().unwrap();
            items.push(item);
            if let Some((count, stop)) = &self.stop_after
                && items.len() == *count
            {
                request_stop(stop);
            }
        }
    }

    #[test]
    fn deterministic_samples_use_settings_and_preserve_inventory_order() {
        let worker = SimulatedEssenceScanWorker::new(essence::ScanSettings {
            skip_abandoned: true,
            non_five_star: essence::NonFiveStar::Skip,
            high_level: Some([3, 3, 3]),
            ..essence::ScanSettings::default()
        });
        let events = Samples::default();
        let reason = worker.scan(&new_stop_token(), &events, Duration::ZERO);
        assert!(matches!(reason, FinishReason::Completed));
        let items = events.items.lock().unwrap();
        assert_eq!(items.len(), 54);
        let reasons: Vec<_> = items
            .iter()
            .take(7)
            .map(|item| item.evaluation.reason)
            .collect();
        assert_eq!(
            reasons,
            [
                essence::Reason::WeaponMatch,
                essence::Reason::NoMatchingWeapon,
                essence::Reason::Locked,
                essence::Reason::Abandoned,
                essence::Reason::NonFiveStar,
                essence::Reason::HighLevel,
                essence::Reason::IncompleteRecognition,
            ]
        );
        assert_eq!(
            (
                items[44].sequence,
                items[44].page,
                items[44].row,
                items[44].column
            ),
            (45, 1, 5, 9)
        );
        assert_eq!(
            (
                items[45].sequence,
                items[45].page,
                items[45].row,
                items[45].column
            ),
            (46, 2, 6, 1)
        );
    }

    #[test]
    fn stop_prevents_all_later_results() {
        let stop = new_stop_token();
        let worker = SimulatedEssenceScanWorker::new(essence::ScanSettings::default());
        let events = Samples {
            stop_after: Some((3, Arc::clone(&stop))),
            ..Samples::default()
        };
        let reason = worker.scan(&stop, &events, Duration::ZERO);
        assert!(matches!(reason, FinishReason::Stopped));
        assert_eq!(events.items.lock().unwrap().len(), 3);
    }
}
