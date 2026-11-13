use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};

/// ลงทะเบียน global hotkey — คืน false ถ้ามีโปรแกรมอื่นจองคอมโบนี้ไว้ก่อน
pub fn register(hwnd: HWND, id: i32, mods: HOT_KEY_MODIFIERS, vk: u32) -> bool {
    unsafe { RegisterHotKey(Some(hwnd), id, mods, vk).is_ok() }
}

pub fn unregister(hwnd: HWND, id: i32) {
    unsafe {
        let _ = UnregisterHotKey(Some(hwnd), id);
    }
}
