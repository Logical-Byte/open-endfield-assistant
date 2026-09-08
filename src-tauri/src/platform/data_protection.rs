//! 与当前操作系统用户绑定的数据保护接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::windows;

/// 使用当前用户的数据保护能力加密字节串；macOS 开发外壳返回 unsupported error。
pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        windows::dpapi::encrypt(plain)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = plain;
        Err(super::unsupported("DPAPI encryption"))
    }
}

/// 使用当前用户的数据保护能力解密字节串；macOS 开发外壳返回 unsupported error。
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        windows::dpapi::decrypt(data)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = data;
        Err(super::unsupported("DPAPI decryption"))
    }
}
