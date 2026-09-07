//! Windows DPAPI protection for branch key at rest.

#[cfg(windows)]
use windows::Win32::Foundation::LocalFree;
#[cfg(windows)]
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
};
#[cfg(windows)]
use windows::core::PCWSTR;

const PREFIX: &str = "dpapi:";

/// Encrypt plaintext with DPAPI (current user). Non-Windows stores plain (dev only).
pub fn protect(plain: &str) -> Result<String, String> {
    if plain.trim().is_empty() {
        return Ok(String::new());
    }
    #[cfg(windows)]
    {
        let bytes = plain.as_bytes();
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptProtectData(
                &mut input,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok.is_err() || output.pbData.is_null() || output.cbData == 0 {
            return Err("No se pudo proteger la clave localmente".into());
        }
        let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, slice);
        unsafe {
            let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(output.pbData as _)));
        }
        Ok(format!("{PREFIX}{encoded}"))
    }
    #[cfg(not(windows))]
    {
        Ok(plain.to_string())
    }
}

/// Decrypt DPAPI payload. Plain values without prefix are returned as-is (migration).
pub fn unprotect(stored: &str) -> Result<String, String> {
    if stored.trim().is_empty() {
        return Ok(String::new());
    }
    if !stored.starts_with(PREFIX) {
        return Ok(stored.to_string());
    }
    #[cfg(windows)]
    {
        let b64 = &stored[PREFIX.len()..];
        let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|_| "La clave protegida no es válida".to_string())?;
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &mut input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok.is_err() || output.pbData.is_null() || output.cbData == 0 {
            return Err("No se pudo leer la clave local. Vuelve a conectarte.".into());
        }
        let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
        let plain = String::from_utf8_lossy(slice).to_string();
        unsafe {
            let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(output.pbData as _)));
        }
        Ok(plain)
    }
    #[cfg(not(windows))]
    {
        Ok(stored.to_string())
    }
}

/// Mask key for logs / diagnostics (never full key).
pub fn mask_key(key: &str) -> String {
    let t = key.trim();
    if t.len() <= 6 {
        return "****".into();
    }
    format!("{}…{}", &t[..3], &t[t.len().saturating_sub(3)..])
}
