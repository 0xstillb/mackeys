use anyhow::{bail, Result};
use windows::core::HSTRING;
use windows::Win32::Foundation::{ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW,
    RegDeleteValueW, RegSetValueExW,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "MacKeys";

/// ให้โปรแกรมรันพร้อม Windows (HKCU\...\Run)
pub fn install() -> Result<()> {
    let exe = std::env::current_exe()?;
    let command = format!("\"{}\"", exe.display());
    write_run_value(&command)
}

/// ยกเลิกการรันพร้อม Windows
pub fn uninstall() -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            None,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            windows::Win32::System::Registry::KEY_QUERY_VALUE
                | windows::Win32::System::Registry::KEY_SET_VALUE,
            None,
            &mut key,
            None,
        );
        if status != ERROR_SUCCESS {
            bail!("เปิดคีย์ Run ไม่ได้ (รหัส {})", status.0);
        }
        let status = RegDeleteValueW(key, &HSTRING::from(VALUE_NAME));
        let _ = RegCloseKey(key);
        if status != ERROR_SUCCESS && status != WIN32_ERROR(2) {
            bail!("ลบค่า {VALUE_NAME} ไม่สำเร็จ (รหัส {})", status.0);
        }
    }
    Ok(())
}

fn write_run_value(command: &str) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            None,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            windows::Win32::System::Registry::KEY_SET_VALUE,
            None,
            &mut key,
            None,
        );
        if status != ERROR_SUCCESS {
            bail!("เปิดคีย์ Run ไม่ได้ (รหัส {})", status.0);
        }

        // REG_SZ ต้องเป็น UTF-16LE ที่มี null terminator
        let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|c| c.to_le_bytes()).collect();

        let status = RegSetValueExW(key, &HSTRING::from(VALUE_NAME), None, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(key);
        if status != ERROR_SUCCESS {
            bail!("เขียนค่า {VALUE_NAME} ไม่สำเร็จ (รหัส {})", status.0);
        }
    }
    Ok(())
}

/// ตรวจว่าตั้งค่ารันพร้อม Windows ไว้อยู่หรือไม่
pub fn is_installed() -> bool {
    use windows::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};
    unsafe {
        let mut size: u32 = 0;
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            &HSTRING::from(VALUE_NAME),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        );
        status == ERROR_SUCCESS && size > 0
    }
}
