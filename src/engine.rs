use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use anyhow::Result;

use crate::action::{Action, ScreenshotMode};
use crate::config::Config;
use crate::hotkey::Hotkey;

/// ธงสำหรับ "กำลัง inject คีย์อยู่" — ระหว่างนี้ให้ข้าม hotkey ที่เพิ่งถูกยิง
///
/// จำเป็นเพราะ: การ inject `Alt+Tab` จะไปโดน hotkey `Ctrl+Tab` ที่ลงทะเบียนไว้
/// (คีย์ที่ inject ก็นับเป็นการกดคีย์ของ Windows เหมือนกัน) → วนลูปไม่จบ
static SUPPRESS_HOTKEYS: AtomicBool = AtomicBool::new(false);

/// เริ่มช่วงที่ห้าม hotkey ทำงาน (ระหว่าง inject คีย์)
pub fn suppress_hotkeys() {
    SUPPRESS_HOTKEYS.store(true, Ordering::SeqCst);
}

/// จบช่วงที่ห้าม hotkey ทำงาน
pub fn resume_hotkeys() {
    SUPPRESS_HOTKEYS.store(false, Ordering::SeqCst);
}

/// ตอนนี้ hotkey ถูกระงับอยู่หรือไม่
pub fn hotkeys_suppressed() -> bool {
    SUPPRESS_HOTKEYS.load(Ordering::SeqCst)
}

/// สถานะการลงทะเบียน hotkey แต่ละรายการ (เอาไปแสดงใน GUI)
#[derive(Debug, Clone)]
pub struct BindingStatus {
    pub keys: String,
    pub label: String,
    pub ok: bool,
    pub message: Option<String>,
}

/// ข้อความจาก win thread → ตัว GUI
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// ให้แสดงหน้าต่างตั้งค่า
    ShowSettings,
    /// ให้ปิดโปรแกรม
    Quit,
    /// สถานะ hotkey เปลี่ยนแล้ว
    StatusChanged,
    /// ข้อความแจ้งผู้ใช้
    Notify(String),
}

/// คำสั่งจาก GUI → win thread
#[derive(Debug, Clone)]
pub enum WinCommand {
    /// โหลด config ใหม่แล้วลงทะเบียน hotkey ใหม่
    ReloadConfig,
    /// ปิดโปรแกรม
    Quit,
    /// ปล่อยปุ่ม modifier ที่ค้างอยู่ (Ctrl/Alt) — แก้อาการ "พิมพ์ไม่ได้"
    ReleaseKeys,
}

pub struct Runtime {
    pub config: RwLock<Config>,
    pub status: RwLock<Vec<BindingStatus>>,
    pub events: std::sync::mpsc::Sender<AppEvent>,
    pub actions: std::sync::mpsc::Sender<Action>,
}

impl Runtime {
    pub fn cycle(&self) -> Vec<String> {
        self.config
            .read()
            .map(|c| c.layouts.cycle.clone())
            .unwrap_or_default()
    }
}

/// ส่ง event ไปหา GUI พร้อมปลุกมันให้อ่าน (สำคัญตอนหน้าต่างถูกซ่อนอยู่)
pub fn emit(rt: &Runtime, event: AppEvent) {
    let _ = rt.events.send(event);
    crate::gui::wake();
}

/// ทำ action ตามที่ผู้ใช้กด — เรียกจาก worker thread เท่านั้น
pub fn execute(rt: &Runtime, action: &Action) {
    let result: Result<String> = match action {
        Action::SwitchLayout(target) => {
            let cycle = rt.cycle();
            crate::win::layout::switch(target, &cycle)
        }
        Action::Screenshot(ScreenshotMode::Full) => crate::win::capture::capture_fullscreen()
            .map(|p| format!("บันทึกภาพแล้ว: {}", p.display())),
        Action::Screenshot(ScreenshotMode::Region) => crate::win::capture::capture_region()
            .map(|_| "เปิดโหมดจับภาพบริเวณแล้ว".to_string()),
        Action::Screenshot(ScreenshotMode::Panel) => {
            crate::win::capture::open_panel().map(|_| "เปิด Snipping Tool แล้ว".to_string())
        }
        Action::AppSwitcher => {
            // ในโหมด ⌘+Tab hook จัดการเองแล้ว (กด ⌘ ค้าง + Tab ไปเรื่อย ๆ)
            // → ถ้ายิงซ้ำจะได้ 2 ครั้งและ task switcher เพี้ยน
            if crate::win::hook::switcher_active() {
                Ok("กำลังใช้ app switcher แบบ ⌘+Tab อยู่".to_string())
            } else {
                // ระหว่าง inject Alt+Tab ต้องกันไม่ให้ไปโดน hotkey Ctrl+Tab ที่ลงทะเบียนไว้
                crate::engine::suppress_hotkeys();
                // **ปล่อย Ctrl ที่โหมด Mac inject ค้างอยู่ก่อน**
                // ไม่งั้นระบบจะเห็นเป็น Ctrl+Alt+Tab → task switcher เปิดผิดโหมด
                // และค้างไว้โดยไม่ปิด (อาการ: กด Alt+Tab แล้วหน้าต่างสลับขึ้นค้าง)
                let suspended = crate::win::hook::suspend_ctrl();
                crate::win::inject::alt_tab();
                std::thread::sleep(std::time::Duration::from_millis(220));
                if suspended {
                    crate::win::hook::resume_ctrl();
                }
                crate::engine::resume_hotkeys();
                Ok("สลับหน้าต่างแล้ว".to_string())
            }
        }
        Action::SendKeys(target) => match Hotkey::parse(target) {
            Ok(hk) => {
                crate::win::inject::send_hotkey(&hk);
                Ok(format!("ส่งคีย์ {target} แล้ว"))
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        },
    };

    match result {
        Ok(msg) => {
            tracing::info!("{msg}");
            emit(rt, AppEvent::Notify(msg));
        }
        Err(e) => {
            tracing::error!("{e:#}");
            emit(rt, AppEvent::Notify(format!("ทำงานไม่สำเร็จ: {e}")));
        }
    }
}

/// แปลงคีย์ลัดใน config ให้เป็น Hotkey พร้อมข้อความ error
#[allow(dead_code)]
pub fn parse_binding(binding: &crate::config::Binding) -> Result<Hotkey, String> {
    Hotkey::parse(&binding.keys)
}
