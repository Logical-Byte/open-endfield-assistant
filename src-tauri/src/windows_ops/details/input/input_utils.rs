use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
};
use windows::Win32::UI::WindowsAndMessaging::{XBUTTON1, XBUTTON2};

use crate::windows_ops::input::Contact;

// MOUSEEVENTF 标志和按钮数据
pub(super) struct MouseEventFlags {
    /// 传给 Win32 鼠标输入 API 的 `MOUSEEVENTF_*` 位模式。
    pub flags: u32,
    pub button_data: u32,
}

// 将 contact ID 转换为 MOUSEEVENTF 按下标志（用于 SendInput/mouse_event）
fn contact_to_mouse_down_flags(contact: Contact) -> MouseEventFlags {
    match contact {
        Contact::Left => MouseEventFlags {
            flags: MOUSEEVENTF_LEFTDOWN.0,
            button_data: 0,
        },
        Contact::Right => MouseEventFlags {
            flags: MOUSEEVENTF_RIGHTDOWN.0,
            button_data: 0,
        },
        Contact::Middle => MouseEventFlags {
            flags: MOUSEEVENTF_MIDDLEDOWN.0,
            button_data: 0,
        },
        Contact::X1 => MouseEventFlags {
            flags: MOUSEEVENTF_XDOWN.0,
            button_data: XBUTTON1 as u32,
        },
        Contact::X2 => MouseEventFlags {
            flags: MOUSEEVENTF_XDOWN.0,
            button_data: XBUTTON2 as u32,
        },
    }
}

// 将 contact ID 转换为 MOUSEEVENTF 抬起标志（用于 SendInput/mouse_event）
fn contact_to_mouse_up_flags(contact: Contact) -> MouseEventFlags {
    match contact {
        Contact::Left => MouseEventFlags {
            flags: MOUSEEVENTF_LEFTUP.0,
            button_data: 0,
        },
        Contact::Right => MouseEventFlags {
            flags: MOUSEEVENTF_RIGHTUP.0,
            button_data: 0,
        },
        Contact::Middle => MouseEventFlags {
            flags: MOUSEEVENTF_MIDDLEUP.0,
            button_data: 0,
        },
        Contact::X1 => MouseEventFlags {
            flags: MOUSEEVENTF_XUP.0,
            button_data: XBUTTON1 as u32,
        },
        Contact::X2 => MouseEventFlags {
            flags: MOUSEEVENTF_XUP.0,
            button_data: XBUTTON2 as u32,
        },
    }
}

impl Contact {
    pub(super) fn to_mouse_down_flags(self) -> MouseEventFlags {
        contact_to_mouse_down_flags(self)
    }

    pub(super) fn to_mouse_up_flags(self) -> MouseEventFlags {
        contact_to_mouse_up_flags(self)
    }
}
