//! 与当前操作系统用户绑定的数据保护接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::windows;

/// 使用当前用户的数据保护能力加密字节串。
#[cfg(target_os = "windows")]
pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>> {
    windows::dpapi::encrypt(plain)
}

/// macOS 开发外壳不提供 Windows DPAPI。
#[cfg(target_os = "macos")]
pub fn encrypt(_plain: &[u8]) -> Result<Vec<u8>> {
    Err(super::unsupported("DPAPI encryption"))
}

/// 使用当前用户的数据保护能力解密字节串。
#[cfg(target_os = "windows")]
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    windows::dpapi::decrypt(data)
}

/// macOS 开发外壳不提供 Windows DPAPI。
#[cfg(target_os = "macos")]
pub fn decrypt(_data: &[u8]) -> Result<Vec<u8>> {
    Err(super::unsupported("DPAPI decryption"))
}
