use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFY_ICON_DATA_FLAGS,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, MENU_ITEM_FLAGS, MF_STRING, MFT_SEPARATOR,
    PostMessageW, SetForegroundWindow, TrackPopupMenu, TRACK_POPUP_MENU_FLAGS, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL,
};

/// ข้อความที่ส่งกลับมายังหน้าต่างเมื่อคลิกไอคอนใน system tray
pub const WM_TRAY: u32 = WM_APP + 1;

pub const CMD_SWITCH_LAYOUT: usize = 1001;
pub const CMD_SETTINGS: usize = 1002;
pub const CMD_OPEN_CONFIG: usize = 1003;
pub const CMD_EXIT: usize = 1004;
/// ปล่อยปุ่มที่ค้างอยู่ (Ctrl/Alt) — ใช้ตอนอาการ "พิมพ์ไม่ได้"
pub const CMD_RELEASE_KEYS: usize = 1005;

pub struct Tray {
    hwnd: HWND,
    /// เก็บใน RefCell เพราะ tooltip เปลี่ยนได้ แต่ต้องอัปเดตได้โดยไม่ต้องถือ `&mut self`
    nid: std::cell::RefCell<NOTIFYICONDATAW>,
    /// เก็บไว้ให้ bitmap ของไอคอนมีอายุตลอดชีวิตของไอคอน
    _icon: crate::win::icon::AppIcon,
}

impl Tray {
    pub fn new(hwnd: HWND, tip: &str) -> Option<Self> {
        unsafe {
            let app_icon = crate::win::icon::create_app_icon()?;

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NOTIFY_ICON_DATA_FLAGS(NIF_ICON.0 | NIF_MESSAGE.0 | NIF_TIP.0),
                uCallbackMessage: WM_TRAY,
                hIcon: app_icon.icon,
                ..Default::default()
            };
            set_wide(&mut nid.szTip, tip);

            if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                tracing::error!(
                    "เพิ่มไอคอนใน system tray ไม่สำเร็จ (hwnd={:?})",
                    hwnd.0
                );
                return None;
            }
            tracing::info!("เพิ่มไอคอนใน system tray สำเร็จ");

            Some(Self {
                hwnd,
                nid: std::cell::RefCell::new(nid),
                _icon: app_icon,
            })
        }
    }

    /// อัปเดต tooltip (แสดงสถานะล่าสุด)
    pub fn set_tip(&self, tip: &str) {
        unsafe {
            let mut nid = self.nid.borrow_mut();
            set_wide(&mut nid.szTip, tip);
            let _ = Shell_NotifyIconW(NIM_MODIFY, &*nid);
        }
    }

    /// แสดงเมนูขวาคลิก แล้วคืนคำสั่งที่ผู้ใช้เลือก
    ///
    /// หมายเหตุ: `TrackPopupMenu` เป็น modal loop ที่ pump ข้อความกลับเข้า
    /// window procedure ของเรา → ผู้เรียกต้อง **ไม่ถือ borrow ของ state** ไว้ตอนนี้
    pub fn show_menu(&self) -> Option<usize> {
        unsafe {
            let menu = CreatePopupMenu().ok()?;

            let _ = AppendMenuW(
                menu,
                MF_STRING,
                CMD_SWITCH_LAYOUT,
                &HSTRING::from("สลับภาษาตอนนี้"),
            );
            let _ = AppendMenuW(menu, separator_flags(), 0, &HSTRING::from(""));
            let _ = AppendMenuW(menu, MF_STRING, CMD_SETTINGS, &HSTRING::from("ตั้งค่า..."));
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                CMD_OPEN_CONFIG,
                &HSTRING::from("เปิดโฟลเดอร์ config"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                CMD_RELEASE_KEYS,
                &HSTRING::from("รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)"),
            );
            let _ = AppendMenuW(menu, separator_flags(), 0, &HSTRING::from(""));
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                CMD_EXIT,
                &HSTRING::from("ออกจากโปรแกรม"),
            );

            let pt = crate::win::util::cursor_pos();
            // ต้อง SetForegroundWindow ก่อน ไม่งั้นเมนูจะไม่ปิดเมื่อคลิกที่อื่น
            let _ = SetForegroundWindow(self.hwnd);

            let cmd = TrackPopupMenu(
                menu,
                TRACK_POPUP_MENU_FLAGS(
                    TPM_RIGHTBUTTON.0 | TPM_RETURNCMD.0 | TPM_NONOTIFY.0,
                ),
                pt.x,
                pt.y,
                None,
                self.hwnd,
                None,
            );

            let _ = DestroyMenu(menu);
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));

            if cmd.0 > 0 {
                Some(cmd.0 as usize)
            } else {
                None
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            let nid = self.nid.borrow();
            let _ = Shell_NotifyIconW(NIM_DELETE, &*nid);
            let _ = DeleteObject(HGDIOBJ(nid.hIcon.0));
        }
    }
}

fn separator_flags() -> MENU_ITEM_FLAGS {
    MENU_ITEM_FLAGS(MFT_SEPARATOR.0)
}

fn set_wide(buf: &mut [u16], text: &str) {
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = match text.encode_utf16().nth(i) {
            Some(c) => c,
            None => 0,
        };
    }
}

/// เหตุการณ์จากการคลิกไอคอน
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// คลิกซ้าย / ดับเบิลคลิก → เปิดหน้าต่างตั้งค่า
    Activate,
    /// คลิกขวา → แสดงเมนูป็อปอัป
    Menu,
}

/// คีย์เมาส์ที่ Windows ส่งมาพร้อม callback ของไอคอน
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_CONTEXTMENU: u32 = 0x007B;
/// NIN_SELECT — ผู้ใช้เลือกไอคอนด้วยคีย์บอร์ด
const NIN_SELECT: u32 = 0x0400;

/// แปลงข้อความ callback ของไอคอนเป็นเหตุการณ์ที่ต้องทำ
pub fn tray_event(lparam: LPARAM) -> Option<TrayEvent> {
    match lparam.0 as u32 {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => Some(TrayEvent::Activate),
        WM_RBUTTONUP | WM_CONTEXTMENU | NIN_SELECT => Some(TrayEvent::Menu),
        _ => {
            let _ = WM_LBUTTONDOWN;
            None
        }
    }
}
