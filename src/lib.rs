use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};

pub mod geometry;
pub mod hotkey;
mod include;
pub mod input;
pub mod ocr;
pub mod screencap;
pub mod template_matching;
pub mod utils;
pub mod window;

pub fn set_thread_dpi_awareness_context() -> DPI_AWARENESS_CONTEXT {
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
}
