use windows::Win32::System::SystemServices::{
    MK_LBUTTON, MK_MBUTTON, MK_RBUTTON, MK_XBUTTON1, MK_XBUTTON2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MapVirtualKeyW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    Left = 0,
    Right = 1,
    Middle = 2,
    X1 = 3,
    X2 = 4,
}

const fn make_message_param(low: u32, high: u32) -> usize {
    ((low & 0xffff) | ((high & 0xffff) << 16)) as usize
}

// Contact 到 WM_* 消息的转换结果
pub struct MouseMessageInfo {
    pub message: u32,
    pub w_param: usize,
}

// 将 contact ID 转换为鼠标按下消息
pub fn contact_to_mouse_down_message(contact: Contact) -> MouseMessageInfo {
    match contact {
        Contact::Left => MouseMessageInfo {
            message: WM_LBUTTONDOWN,
            w_param: MK_LBUTTON.0 as usize,
        },
        Contact::Right => MouseMessageInfo {
            message: WM_RBUTTONDOWN,
            w_param: MK_RBUTTON.0 as usize,
        },
        Contact::Middle => MouseMessageInfo {
            message: WM_MBUTTONDOWN,
            w_param: MK_MBUTTON.0 as usize,
        },
        Contact::X1 => MouseMessageInfo {
            message: WM_XBUTTONDOWN,
            w_param: make_message_param(MK_XBUTTON1.0, XBUTTON1.into()),
        },
        Contact::X2 => MouseMessageInfo {
            message: WM_XBUTTONDOWN,
            w_param: make_message_param(MK_XBUTTON2.0, XBUTTON2.into()),
        },
    }
}

// 将 contact ID 转换为鼠标移动消息
pub fn contact_to_mouse_move_message(contact: Contact) -> MouseMessageInfo {
    match contact {
        Contact::Left => MouseMessageInfo {
            message: WM_MOUSEMOVE,
            w_param: MK_LBUTTON.0 as usize,
        },
        Contact::Right => MouseMessageInfo {
            message: WM_MOUSEMOVE,
            w_param: MK_RBUTTON.0 as usize,
        },
        Contact::Middle => MouseMessageInfo {
            message: WM_MOUSEMOVE,
            w_param: MK_MBUTTON.0 as usize,
        },
        Contact::X1 => MouseMessageInfo {
            message: WM_MOUSEMOVE,
            w_param: MK_XBUTTON1.0 as usize,
        },
        Contact::X2 => MouseMessageInfo {
            message: WM_MOUSEMOVE,
            w_param: MK_XBUTTON2.0 as usize,
        },
    }
}

// 将 contact ID 转换为鼠标抬起消息
pub fn contact_to_mouse_up_message(contact: Contact) -> MouseMessageInfo {
    match contact {
        Contact::Left => MouseMessageInfo {
            message: WM_LBUTTONUP,
            w_param: 0,
        },
        Contact::Right => MouseMessageInfo {
            message: WM_RBUTTONUP,
            w_param: 0,
        },
        Contact::Middle => MouseMessageInfo {
            message: WM_MBUTTONUP,
            w_param: 0,
        },
        Contact::X1 => MouseMessageInfo {
            message: WM_XBUTTONUP,
            w_param: make_message_param(0, XBUTTON1.into()),
        },
        Contact::X2 => MouseMessageInfo {
            message: WM_XBUTTONUP,
            w_param: make_message_param(0, XBUTTON2.into()),
        },
    }
}

// MOUSEEVENTF 标志和按钮数据
pub struct MouseEventFlags {
    pub flags: u32,
    pub button_data: u32,
}

// 将 contact ID 转换为 MOUSEEVENTF 按下标志（用于 SendInput/mouse_event）
pub fn contact_to_mouse_down_flags(contact: Contact) -> MouseEventFlags {
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
pub fn contact_to_mouse_up_flags(contact: Contact) -> MouseEventFlags {
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
    pub fn to_mouse_down_message(self) -> MouseMessageInfo {
        contact_to_mouse_down_message(self)
    }

    pub fn to_mouse_move_message(self) -> MouseMessageInfo {
        contact_to_mouse_move_message(self)
    }

    pub fn to_mouse_up_message(self) -> MouseMessageInfo {
        contact_to_mouse_up_message(self)
    }

    pub fn to_mouse_down_flags(self) -> MouseEventFlags {
        contact_to_mouse_down_flags(self)
    }

    pub fn to_mouse_up_flags(self) -> MouseEventFlags {
        contact_to_mouse_up_flags(self)
    }
}

// 构造 WM_KEYDOWN 的 lParam
pub fn make_keydown_lparam(key: i32) -> isize {
    let sc = unsafe { MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC) };
    (1 | (sc << 16)) as isize
}

// 构造 WM_KEYUP 的 lParam
pub fn make_keyup_lparam(key: i32) -> isize {
    let sc = unsafe { MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC) };
    // 置位先前状态与转换状态位
    (1 | (sc << 16) | (1 << 30) | (1 << 31)) as isize
}
