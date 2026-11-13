use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;

/// กันไม่ให้โปรแกรมรันซ้ำ (ใช้ named mutex ของ session เดียวกัน)
pub struct InstanceGuard(HANDLE);

impl InstanceGuard {
    pub fn acquire(name: &str) -> Option<Self> {
        unsafe {
            let handle = CreateMutexW(None, true, &HSTRING::from(name)).ok()?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                None
            } else {
                Some(Self(handle))
            }
        }
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// ตรวจว่ามี instance อื่นของโปรแกรมกำลังรันอยู่หรือไม่ (ไม่ถือ mutex ไว้)
pub fn is_running(name: &str) -> bool {
    unsafe {
        let Ok(handle) = CreateMutexW(None, false, &HSTRING::from(name)) else {
            return false;
        };
        let already = GetLastError() == ERROR_ALREADY_EXISTS;
        let _ = CloseHandle(handle);
        already
    }
}
