//! 注册表读取工具（仅 Windows）。
//!
//! 使用 `windows` crate 的 `RegGetValueW` 读取注册表值，供 WebView2 Runtime 检测、
//! Windows 系统代理解析等模块复用（取代原 `winreg` crate 的用法）。

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{HKEY, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::PCWSTR;

/// 读取注册表中的字符串值（REG_SZ）。
///
/// 如果键或值不存在，返回 `Ok(None)`。
pub fn read_registry_string(
    hkey: HKEY,
    subkey: &str,
    value_name: &str,
) -> Result<Option<String>, WIN32_ERROR> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut data_size = 0;

    // 1. 第一次调用：获取数据大小。
    // `RRF_RT_REG_SZ` 标志位非常关键，它告诉系统：如果不是字符串，直接报错，并且保证返回的数据自带 `\0` 结尾。
    let status = unsafe {
        RegGetValueW(
            hkey,
            PCWSTR::from_raw(subkey_wide.as_ptr()),
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            RRF_RT_REG_SZ,
            None, // 不需要接收具体类型，因为 RRF_RT_REG_SZ 已经锁死了类型
            None, // 暂不传入缓冲区
            Some(&mut data_size),
        )
    };

    match status {
        ERROR_SUCCESS => {}                      // 成功获取数据大小，继续
        ERROR_FILE_NOT_FOUND => return Ok(None), // 键或值不存在，返回 `Ok(None)`
        WIN32_ERROR(_) => return Err(status),    // 其他错误，返回 `Err`
    }

    // 2. 分配缓冲区 (`data_size` 是字节数)
    let mut buffer: Vec<u16> = vec![0; data_size as usize / 2];

    // 3. 第二次调用：填入数据
    let status = unsafe {
        RegGetValueW(
            hkey,
            PCWSTR::from_raw(subkey_wide.as_ptr()),
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as _), // 传入指针接收数据
            Some(&mut data_size),
        )
    };

    if status != ERROR_SUCCESS {
        return Err(status);
    }

    // 去掉末尾的 `\0`。`data_size` 是字节数，除以 2 得到 UTF-16 单元数；
    // 使用 `saturating_sub` 防止空值（`data_size < 2`）时下溢 panic，
    // 并用 `min` 兜底防止注册表数据异常时切片越界。
    let len = (data_size as usize / 2).saturating_sub(1).min(buffer.len());
    let result = String::from_utf16_lossy(&buffer[..len]);
    Ok(Some(result))
}

/// 读取注册表中的 DWORD 值（REG_DWORD）。
///
/// 如果键或值不存在，返回 `Ok(None)`。
pub fn read_registry_dword(
    hkey: HKEY,
    subkey: &str,
    value_name: &str,
) -> Result<Option<u32>, WIN32_ERROR> {
    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // REG_DWORD 是固定 4 字节，可一次性用栈上缓冲区读取，无需先查询大小。
    let mut value: u32 = 0;
    let mut data_size = std::mem::size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            hkey,
            PCWSTR::from_raw(subkey_wide.as_ptr()),
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as _), // 传入栈上缓冲区接收数据
            Some(&mut data_size),
        )
    };

    match status {
        ERROR_SUCCESS => Ok(Some(value)),
        ERROR_FILE_NOT_FOUND => Ok(None), // 键或值不存在，返回 `Ok(None)`
        WIN32_ERROR(_) => Err(status),    // 其他错误（含类型不匹配）返回 `Err`
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::Foundation::{ERROR_INVALID_HANDLE, ERROR_UNSUPPORTED_TYPE};
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    use super::*;

    #[test]
    fn test_read_registry_string() {
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
        let value_name = "Desktop";

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        match result {
            Ok(Some(value)) => {
                assert!(!value.is_empty());
                let path = std::path::Path::new(&value);
                assert!(path.is_absolute() && path.exists());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_read_registry_string_missing_value() {
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
        let value_name = "NonExistentValue";

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        assert!(result.is_ok_and(|v| v.is_none()));
    }

    #[test]
    fn test_read_registry_string_missing_path() {
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Explorer\NonExistentPath";
        let value_name = "Desktop";

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        assert!(result.is_ok_and(|v| v.is_none()));
    }

    #[test]
    fn test_read_registry_string_missing_default() {
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
        let value_name = "Shell Folders"; // 这是一个子键，不是一个值

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        assert!(result.is_ok_and(|v| v.is_none()));
    }

    #[test]
    fn test_read_registry_string_invalid_root() {
        let hkey = HKEY(0xFFFFFFFFu32 as _); // 不存在的根键
        let subkey = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        let value_name = "Desktop";

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        assert!(matches!(result, Err(ERROR_INVALID_HANDLE)));
    }

    #[test]
    fn test_read_registry_string_not_string() {
        let hkey = HKEY_LOCAL_MACHINE;
        let subkey = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        let value_name = "CurrentMajorVersionNumber";

        let result = read_registry_string(hkey, subkey, value_name);
        dbg!(&result);

        assert!(matches!(result, Err(ERROR_UNSUPPORTED_TYPE)));
    }

    #[test]
    fn test_read_registry_dword() {
        // 系统代理开关（REG_DWORD），未开启时应返回 0 而不是 `Ok(None)`。
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
        let value_name = "ProxyEnable";

        let result = read_registry_dword(hkey, subkey, value_name);
        dbg!(&result);

        assert!(result.is_ok_and(|v| v.is_some()));
    }

    #[test]
    fn test_read_registry_dword_missing_value() {
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
        let value_name = "NonExistentDword";

        let result = read_registry_dword(hkey, subkey, value_name);
        dbg!(&result);

        assert!(result.is_ok_and(|v| v.is_none()));
    }

    #[test]
    fn test_read_registry_dword_type_mismatch() {
        // ProxyServer 是 REG_SZ，用 DWORD 读取应返回类型错误。
        let hkey = HKEY_CURRENT_USER;
        let subkey = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
        let value_name = "ProxyServer";

        let result = read_registry_dword(hkey, subkey, value_name);
        dbg!(&result);

        assert!(matches!(result, Err(ERROR_UNSUPPORTED_TYPE)));
    }
}
