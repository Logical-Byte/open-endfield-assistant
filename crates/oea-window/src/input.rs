use std::{thread, time::Duration};

#[cfg(windows)]
use crate::windows::{
    self,
    input::{InputBlock, State},
};
use crate::{Point, Window};

/// 鼠标按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// 滚轮增量，单位沿用 Windows wheel delta，120 表示一格。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScrollDelta {
    /// 正值向右滚动。
    pub horizontal: i32,
    /// 正值向上滚动。
    pub vertical: i32,
}

/// 不可变的输入策略，可重复绑定不同窗口。默认按住 10ms、尝试激活、关闭真实输入屏蔽。
#[derive(Debug, Clone, Copy)]
pub struct InputPolicy {
    pub(crate) click_hold: Duration,
    pub(crate) key_hold: Duration,
    pub(crate) activate_before_input: bool,
    pub(crate) block_user_input: bool,
}

impl Default for InputPolicy {
    fn default() -> Self {
        Self {
            click_hold: Duration::from_millis(10),
            key_hold: Duration::from_millis(10),
            activate_before_input: true,
            block_user_input: false,
        }
    }
}

impl InputPolicy {
    pub fn builder() -> InputPolicyBuilder {
        InputPolicyBuilder::default()
    }

    /// 轻量绑定，不发送输入、不激活窗口、不屏蔽用户输入。
    pub fn bind(&self, window: Window) -> Input {
        Input {
            policy: *self,
            #[cfg(windows)]
            state: State::new(window),
            #[cfg(not(windows))]
            window,
        }
    }
}

/// 输入策略的构造器，未设置项使用当前默认行为。
#[derive(Debug, Default)]
pub struct InputPolicyBuilder(InputPolicy);

impl InputPolicyBuilder {
    /// 一次 `click` 中按下到抬起的时长，允许为零。
    pub fn click_hold(mut self, duration: Duration) -> Self {
        self.0.click_hold = duration;
        self
    }

    /// 一次 `press_key` 中按下到抬起的时长，允许为零。
    pub fn key_hold(mut self, duration: Duration) -> Self {
        self.0.key_hold = duration;
        self
    }

    /// 每次操作前是否尝试激活目标窗口。关闭后调用方负责维持正确的前台窗口。
    pub fn activate_before_input(mut self, enabled: bool) -> Self {
        self.0.activate_before_input = enabled;
        self
    }

    /// 是否在单次操作内屏蔽整个桌面的真实键鼠输入，包括该操作内部的等待。
    ///
    /// 不覆盖两次方法调用之间的时间。开启失败则操作返回错误，退出时尝试解除。
    pub fn block_user_input(mut self, enabled: bool) -> Self {
        self.0.block_user_input = enabled;
        self
    }

    pub fn build(self) -> InputPolicy {
        self.0
    }
}

/// 绑定窗口的物理输入器。操作会移动系统光标，并可能改变前台窗口。
///
/// 同一按下、移动和释放序列应由同一实例完成。基础事件由调用方配对，
/// 不跟踪未释放的按键。多个实例必须由调用方串行使用，避免交叉持有同一按键。
pub struct Input {
    policy: InputPolicy,
    #[cfg(windows)]
    state: State,
    #[cfg(not(windows))]
    window: Window,
}

impl Input {
    /// 按下鼠标按钮，坐标为客户区物理像素。返回后保持按下，直到调用方释放。
    pub fn mouse_down(&mut self, button: MouseButton, point: Point) -> anyhow::Result<()> {
        self.run(|state| state.mouse_down(button, point))
    }

    /// 移动系统鼠标到客户区物理像素位置，不改变按钮状态。
    pub fn move_mouse(&mut self, point: Point) -> anyhow::Result<()> {
        self.run(|state| state.move_mouse(point))
    }

    /// 释放鼠标按钮，不移动光标。
    pub fn mouse_up(&mut self, button: MouseButton) -> anyhow::Result<()> {
        self.run(|state| state.mouse_up(button))
    }

    /// 按下、按 policy 等待、抬起。游戏响应所需的等待由调用方负责。
    pub fn click(&mut self, button: MouseButton, point: Point) -> anyhow::Result<()> {
        let hold = self.policy.click_hold;
        self.run(|state| {
            state.mouse_down(button, point)?;
            thread::sleep(hold);
            state.mouse_up(button)
        })
    }

    /// 在最近一次成功移动的客户区位置滚动，尚未移动时使用当前客户区中心。
    pub fn scroll(&mut self, delta: ScrollDelta) -> anyhow::Result<()> {
        self.run(|state| state.scroll(delta))
    }

    /// 按下 Windows 虚拟键码，返回后保持按下，直到调用方释放。
    pub fn key_down(&mut self, virtual_key: u16) -> anyhow::Result<()> {
        self.run(|state| state.key_down(virtual_key))
    }

    /// 释放 Windows 虚拟键码。
    pub fn key_up(&mut self, virtual_key: u16) -> anyhow::Result<()> {
        self.run(|state| state.key_up(virtual_key))
    }

    /// 按下虚拟键、按 policy 等待、释放。
    pub fn press_key(&mut self, virtual_key: u16) -> anyhow::Result<()> {
        let hold = self.policy.key_hold;
        self.run(|state| {
            state.key_down(virtual_key)?;
            thread::sleep(hold);
            state.key_up(virtual_key)
        })
    }

    fn run<T>(
        &mut self,
        operation: impl FnOnce(&mut State) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            self.state.prepare(self.policy.activate_before_input)?;
            let mut block = InputBlock::enter(self.policy.block_user_input)?;
            let result = operation(&mut self.state);
            let cleanup = block.release();
            // 保留操作本身的错误，解除失败同时记录诊断。
            match result {
                Ok(value) => cleanup.map(|()| value),
                Err(error) => {
                    if let Err(source) = cleanup {
                        tracing::debug!(error = ?source, "解除输入屏蔽失败");
                    }
                    Err(error)
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (operation, self.window);
            Err(crate::unsupported())
        }
    }
}

// 非 Windows 仍编译同一公开 interface，所有系统操作返回 unsupported error。
#[cfg(not(windows))]
struct State;

#[cfg(not(windows))]
impl State {
    fn mouse_down(&mut self, _: MouseButton, _: Point) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
    fn move_mouse(&mut self, _: Point) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
    fn mouse_up(&mut self, _: MouseButton) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
    fn scroll(&mut self, _: ScrollDelta) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
    fn key_down(&mut self, _: u16) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
    fn key_up(&mut self, _: u16) -> anyhow::Result<()> {
        Err(crate::unsupported())
    }
}
