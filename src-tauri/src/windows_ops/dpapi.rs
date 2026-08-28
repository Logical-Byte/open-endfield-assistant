//! Windows DPAPI 加解密接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::details;

/// 使用当前 Windows 用户的 DPAPI 加密字节串。
#[cfg(target_os = "windows")]
pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>> {
    details::dpapi::encrypt(plain)
}

/// macOS 开发外壳不提供 Windows DPAPI。
#[cfg(target_os = "macos")]
pub fn encrypt(_plain: &[u8]) -> Result<Vec<u8>> {
    Err(super::unsupported("DPAPI encryption"))
}

/// 使用当前 Windows 用户的 DPAPI 解密字节串。
#[cfg(target_os = "windows")]
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    details::dpapi::decrypt(data)
}

/// macOS 开发外壳不提供 Windows DPAPI。
#[cfg(target_os = "macos")]
pub fn decrypt(_data: &[u8]) -> Result<Vec<u8>> {
    Err(super::unsupported("DPAPI decryption"))
}
