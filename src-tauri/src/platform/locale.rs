//! 当前用户的系统界面语言。
pub fn user_interface_language() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        super::windows::locale::user_interface_language()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
