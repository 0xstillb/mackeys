use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassExW, SetTimer, TranslateMessage, MSG, WM_COMMAND, WM_DESTROY, WM_HOTKEY,
    WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

use crate::config::Config;
use crate::engine::{AppEvent, BindingStatus, Runtime, WinCommand};
use crate::hotkey::Hotkey;
use crate::tray::{self, Tray};

const TIMER_POLL: usize = 1;
const POLL_INTERVAL_MS: u32 = 200;

struct WinContext {
    rt: Arc<Runtime>,
    cmd_rx: Receiver<WinCommand>,
    hwnd: HWND,
    tray: Option<Rc<Tray>>,
    registered: Vec<i32>,
    config_path: PathBuf,
}

// อยู่ใน thread เดียวกับ message loop เท่านั้น (ไม่ต้อง Sync)
thread_local! {
    static WIN: RefCell<Option<WinContext>> = RefCell::new(None);
}

/// เปิด thread ที่ดูแล window + system tray + ลงทะเบียน hotkey
pub fn spawn(rt: Arc<Runtime>, cmd_rx: Receiver<WinCommand>, config_path: PathBuf) {
    std::thread::spawn(move || {
        unsafe {
            let Ok(module) = GetModuleHandleW(None) else {
                tracing::error!("เรียก GetModuleHandleW ไม่สำเร็จ");
                return;
            };
            let hinstance = HINSTANCE(module.0);

            let class_name = HSTRING::from("MacKeysHostWindow");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            RegisterClassExW(&wc);

            let hwnd = match CreateWindowExW(
                WS_EX_TOOLWINDOW,
                &class_name,
                &HSTRING::from("MacKeys"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("สร้างหน้าต่างหลักไม่สำเร็จ: {e}");
                    return;
                }
            };
            tracing::info!("สร้างหน้าต่างหลักสำเร็จ hwnd={:?}", hwnd.0);

            WIN.with(|cell| {
                *cell.borrow_mut() = Some(WinContext {
                    rt: rt.clone(),
                    cmd_rx,
                    hwnd,
                    tray: None,
                    registered: Vec::new(),
                    config_path,
                });
            });

            WIN.with(|cell| {
                let mut borrow = cell.borrow_mut();
                if let Some(ctx) = borrow.as_mut() {
                    ctx.tray = Tray::new(hwnd, "MacKeys — กำลังทำงาน").map(Rc::new);
                    ctx.reload();
                }
            });

            SetTimer(Some(hwnd), TIMER_POLL, POLL_INTERVAL_MS, None);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // ถอด hook ออกเมื่อจบโปรแกรม
            crate::win::hook::uninstall();
        }
    });
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // กันไว้ให้พังใน handler ไม่ลากทั้งโปรแกรมลง
    // (wnd_proc เป็น extern "system" → Rust abort ทันทีที่ panic)
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        dispatch(hwnd, msg, wparam, lparam)
    })) {
        Ok(r) => r,
        Err(_) => {
            tracing::error!("พังใน wnd_proc (msg={msg}) — ข้ามไป");
            LRESULT(0)
        }
    }
}

/// จัดการข้อความทั้งหมด
///
/// ข้อความบางอย่าง (เช่นการเปิดเมนู tray) ทำให้เกิด message loop ซ้อน
/// จึง **ต้องปล่อย borrow ของ `WIN` ก่อน** เรียกงานเหล่านั้น
/// ไม่งั้นจะเจอ "RefCell already borrowed" แล้วพังทั้งโปรแกรม
unsafe fn dispatch(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_HOTKEY => {
                let id = (wparam.0 & 0xFFFF) as i32;
                // ระบบเพิ่งได้รับอินพุตจริงจากผู้ใช้ — ใช้เป็นหลักฐานตรวจว่า hook ยังมีชีวิตอยู่
                crate::win::hook::note_input_seen();
                hotkey_fired(id);
                return LRESULT(0);
            }
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as u32;
                run_command(id);
                return LRESULT(0);
            }
            WM_TIMER => {
                poll_commands();
                // ตรวจ/แก้ modifier ที่ค้างอยู่ในระบบ (กันอาการ "พิมพ์ไม่ได้" และ hotkey ใช้ไม่ได้)
                crate::win::hook::tick();
                return LRESULT(0);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }

        if msg == tray::WM_TRAY {
            match tray::tray_event(lparam) {
                Some(tray::TrayEvent::Menu) => {
                    show_tray_menu();
                    return LRESULT(0);
                }
                Some(tray::TrayEvent::Activate) => {
                    run_command(tray::CMD_SETTINGS as u32);
                    return LRESULT(0);
                }
                None => {}
            }
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

/// ดึง `Arc<Runtime>` ออกจาก context (ปล่อย borrow ทันที)
fn runtime() -> Option<Arc<Runtime>> {
    WIN.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().map(|c| c.rt.clone())
    })
}

/// เมนู tray ถูกเก็บเป็น `Rc` เพื่อให้เรียกจากนอก borrow ได้
/// (TrackPopupMenu เป็น modal loop ที่ pump ข้อความกลับมา)
fn show_tray_menu() {
    let tray = WIN.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().and_then(|c| c.tray.clone())
    });
    let Some(tray) = tray else { return };
    if let Some(cmd) = tray.show_menu() {
        run_command(cmd as u32);
    }
}

/// ทำงานเมื่อมีคีย์ลัดถูกกด
fn hotkey_fired(id: i32) {
    // ระหว่างที่เรากำลัง inject คีย์อยู่ ให้ข้าม hotkey (กันวนลูป)
    if crate::engine::hotkeys_suppressed() {
        return;
    }
    let Some(rt) = runtime() else { return };
    let action = {
        let Ok(cfg) = rt.config.read() else { return };
        match cfg.bindings.get((id - 1).max(0) as usize) {
            Some(b) => b.action.clone(),
            None => return,
        }
    };
    tracing::debug!("hotkey id={id} → {action:?}");
    let _ = rt.actions.send(action);
}

/// ทำงานตามคำสั่งจากเมนู tray
fn run_command(id: u32) {
    let Some(rt) = runtime() else { return };
    match id as usize {
        tray::CMD_SWITCH_LAYOUT => {
            let _ = rt
                .actions
                .send(crate::action::Action::SwitchLayout("cycle".into()));
        }
        tray::CMD_SETTINGS => {
            crate::engine::emit(&rt, AppEvent::ShowSettings);
        }
        tray::CMD_OPEN_CONFIG => {
            let path = WIN.with(|cell| {
                let borrow = cell.borrow();
                borrow.as_ref().map(|c| c.config_path.clone())
            });
            if let Some(path) = path {
                let dir = crate::config::dir_of(&path);
                if let Err(e) = crate::win::util::open_folder(&dir) {
                    tracing::error!("เปิดโฟลเดอร์ config ไม่ได้: {e}");
                }
            }
        }
        tray::CMD_RELEASE_KEYS => {
            // ปล่อยปุ่มที่ค้างทั้งหมด (ผู้ใช้อาการ "พิมพ์ไม่ได้" เพราะ Ctrl/Alt ค้าง)
            crate::win::hook::release_all();
            crate::engine::emit(
                &rt,
                AppEvent::Notify("รีเซ็ตปุ่มที่ค้างเรียบร้อยแล้ว".into()),
            );
        }
        tray::CMD_EXIT => {
            crate::engine::emit(&rt, AppEvent::Quit);
        }
        _ => {}
    }
}

/// ดึงคำสั่งที่ส่งมาจาก thread อื่น (GUI)
fn poll_commands() {
    WIN.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(ctx) = borrow.as_mut() else { return };
        while let Ok(cmd) = ctx.cmd_rx.try_recv() {
            match cmd {
                WinCommand::ReloadConfig => ctx.reload(),
                WinCommand::Quit => unsafe {
                    PostQuitMessage(0);
                },
                WinCommand::ReleaseKeys => crate::win::hook::release_all(),
            }
        }
    });
}

impl WinContext {
    /// ลงทะเบียน hotkey ใหม่ทั้งหมดตาม config ปัจจุบัน
    fn reload(&mut self) {
        for id in self.registered.drain(..) {
            crate::win::hotkey_register::unregister(self.hwnd, id);
        }

        let cfg = match self.rt.config.read() {
            Ok(c) => c.clone(),
            Err(_) => return,
        };

        // โหมด Mac: แปลงปุ่ม Alt ฝั่งซ้ายให้เป็น Ctrl (ต้องอยู่บน thread ที่มี message loop)
        crate::win::hook::set_enabled(cfg.mac.enabled);

        let status = self.register_all(&cfg);
        let failed = status.iter().filter(|b| !b.ok).count();

        if let Ok(mut slot) = self.rt.status.write() {
            *slot = status;
        }

        if let Some(t) = self.tray.as_ref() {
            let tip = if failed > 0 {
                format!("MacKeys — มี {failed} คีย์ลัดที่ลงทะเบียนไม่ได้")
            } else {
                "MacKeys — กำลังทำงาน".to_string()
            };
            t.set_tip(&tip);
        }

        crate::engine::emit(&self.rt, AppEvent::StatusChanged);
    }

    fn register_all(&mut self, cfg: &Config) -> Vec<BindingStatus> {
        let mut out: Vec<BindingStatus> = Vec::new();

        if !cfg.general.enabled {
            for b in &cfg.bindings {
                out.push(BindingStatus {
                    keys: b.keys.clone(),
                    label: b.keys.clone(),
                    ok: true,
                    message: Some("ปิดทั้งโปรแกรมอยู่".into()),
                });
            }
            return out;
        }

        for (i, b) in cfg.bindings.iter().enumerate() {
            let id = (i + 1) as i32;

            let hotkey = match Hotkey::parse(&b.keys) {
                Ok(hk) => hk,
                Err(e) => {
                    tracing::warn!("คีย์ลัด \"{}\" ใช้ไม่ได้: {e}", b.keys);
                    out.push(BindingStatus {
                        keys: b.keys.clone(),
                        label: b.keys.clone(),
                        ok: false,
                        message: Some(e),
                    });
                    continue;
                }
            };

            let label = hotkey.label();

            if !b.enabled {
                out.push(BindingStatus {
                    keys: b.keys.clone(),
                    label,
                    ok: true,
                    message: Some("ปิดอยู่".into()),
                });
                continue;
            }

            if crate::win::hotkey_register::register(self.hwnd, id, hotkey.win_mods(), hotkey.vk) {
                self.registered.push(id);
                tracing::info!("ลงทะเบียน {label} สำเร็จ");
                out.push(BindingStatus {
                    keys: b.keys.clone(),
                    label,
                    ok: true,
                    message: None,
                });
            } else {
                tracing::warn!("ลงทะเบียน {label} ไม่สำเร็จ — มีโปรแกรมอื่นจองอยู่");
                out.push(BindingStatus {
                    keys: b.keys.clone(),
                    label,
                    ok: false,
                    message: Some("มีโปรแกรมอื่นจองคอมโบนี้ไว้แล้ว".into()),
                });
            }
        }

        out
    }
}

/// ตรวจสอบว่าคีย์ลัดแต่ละอันลงทะเบียนได้ไหม (ใช้ตอน --diagnose)
pub fn diagnose(cfg: &Config) -> Vec<BindingStatus> {
    let mut out = Vec::new();
    for b in &cfg.bindings {
        let hotkey = match Hotkey::parse(&b.keys) {
            Ok(hk) => hk,
            Err(e) => {
                out.push(BindingStatus {
                    keys: b.keys.clone(),
                    label: b.keys.clone(),
                    ok: false,
                    message: Some(e),
                });
                continue;
            }
        };
        let label = hotkey.label();
        if !b.enabled {
            out.push(BindingStatus {
                keys: b.keys.clone(),
                label,
                ok: true,
                message: Some("ปิดอยู่".into()),
            });
            continue;
        }
        let ok =
            crate::win::hotkey_register::register(HWND::default(), 1, hotkey.win_mods(), hotkey.vk);
        if ok {
            crate::win::hotkey_register::unregister(HWND::default(), 1);
        }
        out.push(BindingStatus {
            keys: b.keys.clone(),
            label,
            ok,
            message: if ok {
                None
            } else {
                Some("มีโปรแกรมอื่นจองคอมโบนี้ไว้แล้ว".into())
            },
        });
    }
    out
}
