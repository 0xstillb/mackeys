use anyhow::{bail, Result};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
};

const SW_SHOWNORMAL: i32 = 1;

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        hwnd: HWND,
        lpoperation: PCWSTR,
        lpfile: PCWSTR,
        lpparameters: PCWSTR,
        lpdirectory: PCWSTR,
        nshowcmd: i32,
    ) -> windows::Win32::Foundation::HANDLE;
}

/// หน้าต่างที่กำลังโฟกัสอยู่จริง (ไม่ใช่แค่หน้าต่าง foreground)
pub fn focused_window() -> HWND {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return fg;
        }
        let tid = GetWindowThreadProcessId(fg, None);
        let mut gti = GUITHREADINFO::default();
        if GetGUIThreadInfo(tid, &mut gti).is_ok() && !gti.hwndFocus.0.is_null() {
            return gti.hwndFocus;
        }
        fg
    }
}

pub fn foreground_thread_id() -> u32 {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return 0;
        }
        GetWindowThreadProcessId(fg, None)
    }
}

pub fn cursor_pos() -> POINT {
    unsafe {
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        p
    }
}

/// เปิด URI ผ่าน shell เช่น "ms-screensketch:"
pub fn shell_open(uri: &str) -> Result<()> {
    unsafe {
        let operation = HSTRING::from("open");
        let file = HSTRING::from(uri);
        let empty = HSTRING::from("");
        let result = ShellExecuteW(
            HWND::default(),
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(empty.as_ptr()),
            PCWSTR(empty.as_ptr()),
            SW_SHOWNORMAL,
        );
        // ShellExecuteW คืนค่า <= 32 เมื่อเกิดข้อผิดพลาด
        if (result.0 as isize) <= 32 {
            bail!("เปิด \"{uri}\" ไม่สำเร็จ (รหัส {})", result.0 as isize);
        }
    }
    Ok(())
}

/// เปิดโฟลเดอร์ใน File Explorer
pub fn open_folder(path: &std::path::Path) -> Result<()> {
    shell_open(&path.to_string_lossy())
}

/// ผูก console ของ process พ่อ เพื่อให้พิมพ์ผลลง terminal ได้ตอนรันด้วย --diagnose ฯลฯ
pub fn attach_parent_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
