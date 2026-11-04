use crate::{Window, WindowQuery};
use ::windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use ::windows::core::PCWSTR;
use anyhow::ensure;

pub(crate) fn search(query: WindowQuery<'_>) -> anyhow::Result<Option<Window>> {
    let wide = |value: &str| -> anyhow::Result<Vec<u16>> {
        ensure!(!value.contains('\0'), "窗口查找条件包含空字符");
        Ok(value.encode_utf16().chain(std::iter::once(0)).collect())
    };
    let class = query.class_name.map(wide).transpose()?;
    let title = query.title.map(wide).transpose()?;
    let class_ptr = class.as_ref().map(|value| PCWSTR(value.as_ptr()));
    let title_ptr = title.as_ref().map(|value| PCWSTR(value.as_ptr()));
    // `FindWindowW` 未找到时返回空句柄，且不会设置 last-error。
    match unsafe { FindWindowW(class_ptr.as_ref(), title_ptr.as_ref()) } {
        Ok(raw) if !raw.is_invalid() => Ok(Some(Window { raw })),
        _ => Ok(None),
    }
}
