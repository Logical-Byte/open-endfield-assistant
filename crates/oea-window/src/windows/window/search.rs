use crate::{TitleMatch, Window, WindowQuery};
use ::windows::Win32::Foundation::{HWND, LPARAM};
use ::windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetWindowTextW, IsWindowVisible,
};
use ::windows::core::{BOOL, PCWSTR};
use anyhow::ensure;

pub(crate) fn search(query: WindowQuery<'_>) -> anyhow::Result<Option<Window>> {
    match query.title {
        // 精确标题可以交给系统直接查找，避免枚举全部顶层窗口。
        Some(TitleMatch::Exact(title)) => find_exact(query.class_name, title),
        _ => find_first_matching(query),
    }
}

fn find_exact(class_name: Option<&str>, title: &str) -> anyhow::Result<Option<Window>> {
    let class = class_name.map(wide).transpose()?;
    let title = wide(title)?;
    let class_ptr = class.as_ref().map(|value| PCWSTR(value.as_ptr()));
    let title_ptr = Some(PCWSTR(title.as_ptr()));
    // `FindWindowW` 未找到时返回空句柄，且不会设置 last-error。
    match unsafe { FindWindowW(class_ptr.as_ref(), title_ptr.as_ref()) } {
        Ok(raw) if !raw.is_invalid() => Ok(Some(Window { raw })),
        _ => Ok(None),
    }
}

fn find_first_matching(query: WindowQuery<'_>) -> anyhow::Result<Option<Window>> {
    let mut state = Search {
        class_name: query.class_name,
        title: query.title,
        found: None,
    };
    let state_ptr = (&raw mut state) as isize;
    unsafe { EnumWindows(Some(enum_windows), LPARAM(state_ptr)) }?;
    Ok(state.found.map(|raw| Window { raw }))
}

struct Search<'a> {
    class_name: Option<&'a str>,
    title: Option<TitleMatch<'a>>,
    found: Option<HWND>,
}

/// 枚举顶层窗口。只考虑可见窗口，避免命中隐藏的后台窗口。
///
/// 回调始终返回 `TRUE` 继续枚举：`EnumWindows` 把回调返回 `FALSE` 视为失败，
/// 会读取可能无关的 last-error，无法用来表示 “已找到并提前结束”。
unsafe extern "system" fn enum_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam.0 as *mut Search<'_>) };
    if state.found.is_none() && unsafe { IsWindowVisible(hwnd) }.as_bool() && matches(state, hwnd) {
        state.found = Some(hwnd);
    }
    BOOL(1)
}

fn matches(state: &Search<'_>, hwnd: HWND) -> bool {
    if let Some(expected) = state.class_name
        && window_class(hwnd) != expected
    {
        return false;
    }
    match state.title {
        None => true,
        Some(TitleMatch::Exact(expected)) => window_title(hwnd) == expected,
        Some(TitleMatch::Contains(expected)) => window_title(hwnd).contains(expected),
    }
}

fn window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetWindowTextW(hwnd, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}

fn window_class(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}

fn wide(value: &str) -> anyhow::Result<Vec<u16>> {
    ensure!(!value.contains('\0'), "窗口查找条件包含空字符");
    Ok(value.encode_utf16().chain(std::iter::once(0)).collect())
}
