//! 符号化游戏自动化动作，以及解释这些动作的执行契约。

use crate::utils::region::Region2D;

/// 自动化使用的 1280x720 归一化坐标点。
///
/// 该坐标空间与实际游戏窗口分辨率无关。执行器会根据当前会话的分辨率
/// 将坐标缩放后再发送鼠标输入，因此场景声明不需要了解窗口的实际大小。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point720p {
    /// 水平坐标，取值基于 1280 像素宽的参考画布。
    pub x: u32,
    /// 垂直坐标，取值基于 720 像素高的参考画布。
    pub y: u32,
}

/// 自动化动作使用的逻辑键。
///
/// 这里不保存 Windows 虚拟键码。平台相关的键码转换属于具体执行器，
/// 这样场景定义可以保持平台无关。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Escape 键，用于关闭当前界面或从大世界打开协议终端等场景转换。
    Escape,
}

/// 在归一化坐标空间中描述一次模板搜索目标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemplateTarget {
    /// 模板逻辑名称，例如 `"情报档案库/音像存档.png"`。
    pub template_name: &'static str,
    /// 只在该 720p 基准区域内搜索模板，避免其他界面元素产生误匹配。
    pub roi: Region2D<u32>,
    /// 模板匹配被视为成功所需达到的最低分数。
    pub threshold: f32,
}

/// “对外部游戏世界执行一个副作用”的符号描述。
///
/// 只有将动作交给 [`AutomateExecutor::execute`] 后，具体执行器才会解释并
/// 产生对应的外部副作用。动作值因此可以在执行前被检查、记录或重新排序。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutomateAction {
    /// 点击归一化坐标中的固定位置。
    ///
    /// 执行器负责将 [`Point720p`] 缩放到实际窗口分辨率，并发送一次鼠标左键
    /// 点击；点击完成后会把鼠标移回安全位置，减少 hover 状态对后续识别的影响。
    ClickAt(Point720p),
    /// 按下并释放一个逻辑键。
    ///
    /// 动作只表达按键意图，不携带平台键码。执行器负责把 [`Key`] 转换为
    /// 当前平台的输入表示后发送一次完整的按下、释放操作。
    PressKey(Key),
    /// 在指定区域查找模板，找到后点击匹配区域的中心。
    ///
    /// 执行器会在解释此动作时获取一张新的识别帧，再按 [`TemplateTarget`] 的
    /// 名称、区域和阈值进行匹配。没有达到阈值时不发送点击，并返回
    /// [`ActionOutcome::TargetNotFound`]；匹配成功后才执行中心点点击。
    FindAndClickTemplate(TemplateTarget),
}

/// 解释自动化动作后的业务结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcome {
    /// 动作已经完成，包含固定点击、按键或模板匹配后的点击。
    Applied,
    /// 模板动作没有找到达到阈值的目标，因此没有发送点击。
    ///
    /// 这不是基础设施错误；调用方可以据此决定是否采用其他动作或结束当前
    /// 流程。截图、模板加载或输入设备失败仍会通过 `Result` 返回错误。
    TargetNotFound,
}

/// 解释并执行符号化自动化动作的接口。
///
/// 接口不依赖 [`crate::session::Session`] 的具体实现，因而可以由生产环境的
/// `AutomateContext` 实现，也可以由记录器或测试替身实现。
pub trait AutomateExecutor {
    /// 执行一个动作，并返回动作是否实际应用或未找到模板目标。
    fn execute(&mut self, action: &AutomateAction) -> anyhow::Result<ActionOutcome>;
}
