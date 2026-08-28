//! Mirror酱 CDK 的 DPAPI 加解密模块。
//!
//! 使用 Windows DPAPI（`CryptProtectData` / `CryptUnprotectData`）以**当前用户**作用域加密，
//! 密文仅能在本机、当前 Windows 用户下解密，复制到其他机器或切换用户后均无法解密。
//! 本模块只处理原始字节，Base64 编码交由调用方处理。

use anyhow::Result;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::core::PCWSTR;

/// 用 DPAPI（当前用户作用域）加密字节串，返回密文字节。
pub(in crate::windows_ops) fn encrypt(plain: &[u8]) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    }?;
    // 拷贝出密文后立即释放 DPAPI 分配的内存。
    let encrypted =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(Some(HLOCAL(output.pbData as *mut core::ffi::c_void))) };
    Ok(encrypted)
}

/// 解密密文字节串，返回明文字节串。换机器或切换用户时返回 `Err`。
pub(in crate::windows_ops) fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    }?;
    let plain =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(Some(HLOCAL(output.pbData as *mut core::ffi::c_void))) };
    Ok(plain)
}
