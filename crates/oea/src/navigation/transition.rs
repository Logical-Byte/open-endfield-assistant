//! 导航图中使用的声明式操作与跳转。
//!
//! 本模块只描述操作和可能的结果。实际的截图、模板匹配和输入由 `executor` 解释，
//! 使导航图能够作为无副作用的数据参与策略规划。

use std::time::Duration;

use crate::automation::{Key, Point720p, TemplateTarget};

use super::state::UiState;

/// 导航 `transition` 使用的有限操作词汇。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Op {
    /// 点击一个基于 1280×720 基准分辨率的固定坐标。
    Click(Point720p),
    /// 按下并释放一个键盘按键。
    PressKey(Key),
    /// 用于显式需要等待的复合动作；当前生产图暂时没有此类 `transition`。
    #[allow(dead_code)]
    Sleep(Duration),
    /// 在当前截图的指定区域查找模板，并点击匹配区域的中心。
    ///
    /// 未找到模板且 `fallback` 为 `Some` 时点击该备用坐标。
    /// 既没有找到模板也没有指定 `fallback` 时返回错误。截图、模板匹配和
    /// 点击能力返回的技术错误会直接传播。
    FindAndClickTemplate {
        /// 模板名称、搜索区域和最低匹配得分。
        target: TemplateTarget,
        /// 模板未匹配时可以直接点击的 720p 基准坐标。
        fallback: Option<Point720p>,
    },
}

/// 一个来源状态可执行的声明式跳转。
///
/// `outcomes` 必须列出操作成功后所有可能被识别到的具体状态；
/// [`super::graph::NavigationGraph`] 构造时验证它非空且不含重复状态。
#[derive(Debug, Clone, Copy)]
pub(super) struct Transition<'a> {
    pub(super) ops: &'a [Op],
    pub(super) outcomes: &'a [UiState],
}
