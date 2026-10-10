use windows::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
use windows::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;
pub(in crate::platform) fn user_interface_language() -> Option<String> {
    let language = unsafe { GetUserDefaultUILanguage() };
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    let length = unsafe { LCIDToLocaleName(u32::from(language), Some(&mut buffer), 0) };
    (length > 0).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
}
