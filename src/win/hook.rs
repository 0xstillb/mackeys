//! "โหมด Mac" — แปลงปุ่ม `Alt` ฝั่งซ้ายให้ทำหน้าที่เป็น `Ctrl` (คีย์ ⌘ ของ macOS)
//!
//! ทำงานด้วย low-level keyboard hook (`WH_KEYBOARD_LL`):
//! - กด Alt ซ้าย → กลืนไว้ และ "ล้าง" สถานะ Alt ในระบบ
//! - กดปุ่มอื่นตอน Alt ค้าง → inject `Ctrl` ลง แล้วปล่อยปุ่มนั้นผ่าน
//!   → แอปที่ได้รับเห็นเป็น `Ctrl+<ปุ่ม>` พอดี (เหมือนกด ⌘ บน macOS)
//! - ปล่อย Alt → ปล่อย `Ctrl`
//!
//! ตรรกะทั้งหมดอยู่ใน [`crate::win::hook_state`] (pure, เทสต์ได้) ไฟล์นี้เป็นแค่
//! ตัวห่อ Win32 — ถ้าจะแก้พฤติกรรมให้ไปแก้ที่ `hook_state.rs` แล้วรันเทสต์
//!
//! ข้อควรระวัง: hook ต้องคืนค่าเร็ว ไม่งั้น Windows จะถอด hook ออกเอง
//! (ค่า `LowLevelHooksTimeout` เริ่มต้น 300 ms)

use std::cell::RefCell;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use windows::Win32::Foundation::{LRESULT, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS, LLKHF_INJECTED,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

use super::hook_state::{
    Effect, Event, HookState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_MENU, VK_RCONTROL,
    VK_RMENU, VK_SPACE, VK_TAB,
};

/// ตัวระบุที่เราใส่ไว้ใน `dwExtraInfo` ของอินพุตที่ MacKeys inject เอง
/// เพื่อแยกออกจากอินพุตที่โปรแกรมอื่น inject (เช่น AutoHotkey)
pub(crate) const INJECT_MARKER: usize = 0x4D4B;

/// ตัวระบุของอินพุตที่ "ต้องปล่อยผ่าน" ให้ระบบเห็นจริง ๆ
///
/// ใช้กับ task switcher: เราต้องกด `Alt` จริงให้ Windows เปิด task switcher
/// ถ้าใช้ `INJECT_MARKER` ปกติ hook จะกลืนทิ้ง → task switcher ไม่เปิด
/// การปล่อยผ่านแบบนี้ยังถูกนับว่า "เป็นของเรา" → ไม่ต้องกลัวว่าจะถูกแปลงซ้ำ
pub(crate) const INJECT_PASS_MARKER: usize = 0x4D4C;

/// เวลา (มิลลิวินาที) ที่ hook เห็นเหตุการณ์คีย์ล่าสุด
static LAST_HOOK_EVENT_MS: AtomicU64 = AtomicU64::new(0);

/// เวลาที่ระบบได้รับอินพุตจริงจากผู้ใช้ (เช่นตอน hotkey ยิง)
/// ใช้ตรวจว่า hook ยังทำงานอยู่หรือถูก Windows ถอดไปแล้ว
static LAST_INPUT_MS: AtomicU64 = AtomicU64::new(0);

/// เรา inject `Ctrl` ค้างอยู่ในระบบหรือไม่
/// (action อื่นต้องรู้ เพื่อปล่อย Ctrl ก่อน inject ปุ่มที่ต้องการ modifier สะอาด)
static CTRL_INJECTED: AtomicBool = AtomicBool::new(false);

/// กำลัง "พัก" `Ctrl` ที่ inject ไว้ (ถูกปล่อยไปแล้ว รอกดกลับ)
static CTRL_SUSPENDED: AtomicBool = AtomicBool::new(false);

/// ตอนนี้ task switcher ที่เราเปิดไว้ยังค้างอยู่หรือไม่
///
/// ใช้กันไม่ให้ hotkey `Ctrl+Tab` (ที่ลงทะเบียนไว้) ยิง `Alt+Tab` ซ้ำกับ hook
/// ในโหมด ⌘+Tab ของเรา
static SWITCHER_ACTIVE: AtomicBool = AtomicBool::new(false);

/// ตอนนี้ task switcher ที่ hook เปิดไว้ยังค้างอยู่หรือไม่
pub fn switcher_active() -> bool {
    SWITCHER_ACTIVE.load(Ordering::SeqCst)
}

/// ถ้า hook ไม่เห็นเหตุการณ์เลย นานกว่านี้หลังจากที่ระบบเห็นอินพุต
/// → เดาว่า hook ถูก Windows ถอดแล้ว → ติดตั้งใหม่
const HOOK_DEAD_MS: u64 = 2_000;

/// กันไม่ให้ log "ปุ่มค้าง" ซ้ำทุก 200 ms (เคยทำให้ log รกและเปลือง CPU)
static WARNED_STUCK_CTRL: AtomicBool = AtomicBool::new(false);
static WARNED_STUCK_ALT: AtomicBool = AtomicBool::new(false);

/// จุดเริ่มต้นของเวลา (มิลลิวินาที) — ใช้คำนวณเวลาแบบ monotonic
fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// เวลาปัจจุบันเป็นมิลลิวินาทีนับจากจุดเริ่มต้น
fn now_ms() -> u64 {
    epoch().elapsed().as_millis() as u64
}

/// โหมดทดสอบ: ให้ถือว่าอินพุตที่ถูก inject ก็เป็นปุ่มจริง
/// (ใช้ตอนทดสอบอัตโนมัติ เพราะเครื่องมือส่งคีย์ทุกชนิดเป็น injected เสมอ)
fn test_injected() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| parse_test_flag(std::env::var("MACKEYS_TEST_INJECTED").ok().as_deref()))
}

/// ตัดสินว่าควรปล่อย `Ctrl` ที่ "ค้าง" ในระบบหรือไม่ (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// `system_ctrl_down` = ระบบเห็นว่า Ctrl ถูกกดอยู่
/// `phys_ctrl` = ผู้ใช้กด Ctrl จริง
/// `we_injected` = เราเป็นคน inject Ctrl ที่ยังกดอยู่
///
/// **ต้องใช้ "เรา inject อยู่จริง" (ธง global) ไม่ใช่ธงใน state machine**
/// เพราะตอน `suspend_ctrl()` (ก่อนส่ง Alt+Tab) state machine ยังคิดว่า Ctrl ถูกกดอยู่
/// แต่จริง ๆ เราปล่อยไปแล้ว → ถ้าใช้ธงใน state machine จะไปปล่อย Ctrl ที่ผู้ใช้เพิ่งกดคืน
fn should_release_stuck_ctrl(system_ctrl_down: bool, phys_ctrl: bool, we_injected: bool) -> bool {
    system_ctrl_down && !phys_ctrl && !we_injected
}

/// ตัดสินว่าควรล้าง state machine ทิ้งหรือไม่ (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// เรา inject Ctrl ไว้ แต่ระบบไม่เห็นว่ากดอยู่ → flag เก่า (ค้างคิว) → ล้าง
///
/// ระหว่าง `suspend_ctrl()` (ก่อนส่ง Alt+Tab) ระบบจะไม่เห็น Ctrl ทั้งที่เรายัง "คิด" ว่ากดอยู่
/// → **ต้องไม่ล้าง state** ตอนนั้น ไม่งั้นโหมด Mac จะลืมว่าผู้ใช้กด ⌘ อยู่
fn should_reset_state(system_ctrl_down: bool, we_injected: bool) -> bool {
    we_injected && !system_ctrl_down
}

/// ตีความค่าของ `MACKEYS_TEST_INJECTED` (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// ต้องอ่านค่าจริง ไม่ใช่แค่ดูว่าตัวแปรมีอยู่ — ไม่งั้นการตั้ง `=0` ก็ยังเปิดโหมดทดสอบ
fn parse_test_flag(raw: Option<&str>) -> bool {
    match raw {
        None => false,
        Some(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        ),
    }
}

/// ตัดสินว่า hook "ตาย" หรือยัง (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// หลักฐานว่า hook ตาย: ระบบเห็นอินพุตจากผู้ใช้ (hotkey ยิงได้) แต่ hook ไม่เห็นเหตุการณ์เลย
/// → Windows ถอด low-level hook ทิ้งเอง (เพราะ callback ช้าเกิน `LowLevelHooksTimeout`)
/// → โหมด Mac "ตายเงียบ" จนกว่าจะเปิดโปรแกรมใหม่ (อาการ: "กดไม่เปลี่ยน")
///
/// `input_ms` = เวลาที่ระบบเห็นอินพุต, `hook_ms` = เวลาที่ hook เห็นเหตุการณ์ล่าสุด
fn hook_is_dead(input_ms: u64, hook_ms: u64, dead_after_ms: u64) -> bool {
    if input_ms == 0 {
        // ยังไม่เคยเห็นอินพุตเลย → ยังพิสูจน์อะไรไม่ได้
        return false;
    }
    input_ms.saturating_sub(hook_ms) > dead_after_ms
}

thread_local! {
    /// สถานะของโหมด Mac (logic อยู่ใน `hook_state.rs`)
    static STATE: RefCell<HookState> = RefCell::new(HookState::new());
}

thread_local! {
    /// ตัวจับ hook ที่ติดตั้งไว้ (เก็บไว้เพื่อถอดออกตอนจบ)
    static HOOK: RefCell<Option<HHOOK>> = const { RefCell::new(None) };
}

thread_local! {
    /// บัฟเฟอร์ trace event — **ห้ามเขียนไฟล์ใน hook callback**
    ///
    /// hook callback อยู่ใน "input path" ของ Windows: ถ้าเขียนไฟล์ตรงนี้
    /// (แม้แต่ trace ระดับเดียว) จะทำให้ hook คืนค่าช้า → Windows ส่งอินพุตถัดไปช้าตาม
    /// → ผู้ใช้รู้สึก "คีย์บอร์ดค้าง/หน่วง" ทั้งที่ CPU ยังว่าง
    ///
    /// วิธีแก้: hook แค่พุกไว้ในคิว → timer ของ thread หลักเป็นคนเอาไปเขียน log
    static TRACE_BUF: RefCell<std::collections::VecDeque<String>> =
        const { RefCell::new(std::collections::VecDeque::new()) };
}

/// จำนวน trace event สูงสุดที่เก็บในคิว (กันความจำบวม)
const TRACE_BUF_MAX: usize = 512;

/// จำนวน trace event ที่เขียนต่อรอบของ timer (จำกัดเวลาที่บล็อก thread หลัก)
const TRACE_FLUSH_BATCH: usize = 32;

/// พุก trace event ไว้ในคิว (ไม่เขียนไฟล์ — ดู `flush_trace`)
fn trace_push(msg: String) {
    let mut dropped = 0u32;
    TRACE_BUF.with(|b| {
        let mut b = b.borrow_mut();
        if b.len() >= TRACE_BUF_MAX {
            b.pop_front();
            dropped = 1;
        }
        b.push_back(msg);
    });
    if dropped > 0 {
        // คิวล้น = มีอะไรมากกว่าที่ flush ทัน → ต้องบอกให้รู้ ไม่งั้นจะ "เงียบ" ไปเฉย ๆ
        // (เคยเป็นสาเหตุที่วินิจฉัยยาก เพราะเหมือน hook ตาย)
        tracing::warn!("คิว trace ล้น — ทิ้งรายการเก่า (hook อาจทำงานช้าลง)");
    }
}

/// เอา trace event ที่สะสมมาเขียน log — เรียกจาก timer (นอก hot path)
fn flush_trace() {
    let events: Vec<String> = TRACE_BUF.with(|b| {
        let mut b = b.borrow_mut();
        let n = TRACE_FLUSH_BATCH.min(b.len());
        b.drain(..n).collect()
    });
    for e in events {
        tracing::trace!("{e}");
    }
}

fn key(vk: u32, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: INJECT_MARKER,
            },
        },
    }
}

/// เหมือน [`key`] แต่ใส่ marker ของ "ปล่อยผ่าน" (ต้องให้ระบบเห็นจริง)
fn key_pass(vk: u32, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: INJECT_PASS_MARKER,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// กด/ปล่อย Ctrl ซ้าย
fn inject_ctrl(up: bool) {
    send(&[key(VK_LCONTROL, up)]);
    CTRL_INJECTED.store(!up, Ordering::SeqCst);
    if up {
        // hook ปล่อย Ctrl เอง → ถ้า thread อื่นกำลัง "พัก" Ctrl อยู่ ให้ยกเลิก
        // (ไม่งั้นจะกด Ctrl กลับคืนทั้งที่ผู้ใช้ปล่อย ⌘ ไปแล้ว → Ctrl ค้างในระบบ)
        CTRL_SUSPENDED.store(false, Ordering::SeqCst);
    }
}

/// ปล่อย Alt ซ้าย — ใช้ล้าง "สถานะ Alt" ที่ระบบบันทึกไว้
///
/// เหตุผล: Windows อัปเดต async key state **ก่อน** เรียก low-level hook
/// ถ้าไม่ล้าง ระบบจะยัง "เห็น" ว่า Alt กดอยู่ → `RegisterHotKey` จะไม่ยอมรับ
/// คอมโบ `Ctrl+Space` เพราะเห็น modifier เกิน (Ctrl+Alt+Space)
fn inject_alt_release() {
    send(&[key(VK_LMENU, true)]);
}

/// ปล่อย `Ctrl` ที่เรา inject ค้างอยู่ (ถ้ามี) — ใช้ก่อน inject ปุ่มอื่นที่ต้องการ
/// modifier "สะอาด" เช่น `Alt+Tab` ของ app switcher
///
/// คืนค่า `true` ถ้าปล่อยให้ (ผู้เรียกต้องเรียก `resume_ctrl` คืนทีหลัง)
pub fn suspend_ctrl() -> bool {
    if !CTRL_INJECTED.load(Ordering::SeqCst) {
        return false;
    }
    send(&[key(VK_LCONTROL, true)]);
    CTRL_INJECTED.store(false, Ordering::SeqCst);
    CTRL_SUSPENDED.store(true, Ordering::SeqCst);
    true
}

/// กด `Ctrl` กลับคืนหลังจาก `suspend_ctrl`
///
/// จะไม่กดกลับถ้า hook ปล่อย Ctrl ไปแล้ว (ผู้ใช้ปล่อย ⌘ ก่อนหน้านั้น)
/// — การกดกลับทั้งที่ไม่มีใครมาปล่อย → `Ctrl` ค้างในระบบทั้งระบบ
pub fn resume_ctrl() {
    if !CTRL_SUSPENDED.swap(false, Ordering::SeqCst) {
        return;
    }
    send(&[key(VK_LCONTROL, false)]);
    CTRL_INJECTED.store(true, Ordering::SeqCst);
}

/// ทำ effect ที่ state machine สั่งมา
fn apply(effect: Effect) {
    match effect {
        Effect::CtrlDown => inject_ctrl(false),
        Effect::CtrlUp => inject_ctrl(true),
        Effect::AltUp => inject_alt_release(),
        // task switcher: ต้อง "ปล่อยผ่าน" ให้ Windows เห็นจริง ๆ
        // (ถ้ากลืนไว้ task switcher จะไม่เปิด / ไม่เลื่อน)
        Effect::SwitcherAltDown => {
            SWITCHER_ACTIVE.store(true, Ordering::SeqCst);
            send(&[key_pass(VK_LMENU, false)]);
        }
        Effect::SwitcherTab => send(&[key_pass(VK_TAB, false), key_pass(VK_TAB, true)]),
        Effect::SwitcherAltUp => {
            SWITCHER_ACTIVE.store(false, Ordering::SeqCst);
            send(&[key_pass(VK_LMENU, true)]);
        }
        // ⌘+Space → Win+Space ให้ Windows สลับภาษาเอง (ทำงานทุกที่ ไม่ต้องอยู่ในช่องพิมพ์)
        Effect::WinSpace => send(&[
            key_pass(VK_LWIN, false),
            key_pass(VK_SPACE, false),
            key_pass(VK_SPACE, true),
            key_pass(VK_LWIN, true),
        ]),
    }
}

/// ตอนนี้ `Ctrl` ถูกกดอยู่จริงหรือไม่ (ดูจากสถานะของระบบ)
fn ctrl_is_down() -> bool {
    unsafe {
        [VK_CONTROL, VK_LCONTROL, VK_RCONTROL]
            .iter()
            .any(|&vk| (GetAsyncKeyState(vk as i32) as i32) & 0x8000 != 0)
    }
}

/// ตอนนี้ `Alt` ถูกกดอยู่จริงหรือไม่
///
/// ดูเฉพาะซ้าย/ขวา ไม่ดู `VK_MENU` (ตัวรวม) — เพราะ generic อาจค้างจากการ inject
/// ครั้งก่อน (การกด `VK_MENU` แล้วปล่อยแค่ generic ไม่ล้างสถานะซ้าย/ขวา)
/// ทำให้เข้าใจผิดว่า Alt ค้าง แล้วปล่อย Alt ของผู้ใช้ทิ้ง
fn alt_side_down() -> bool {
    unsafe {
        [VK_LMENU, VK_RMENU]
            .iter()
            .any(|&vk| (GetAsyncKeyState(vk as i32) as i32) & 0x8000 != 0)
    }
}

/// ปล่อย `Ctrl`/`Alt` ที่ค้างอยู่ในระบบ **ให้ครบทุกตัวแปร**
///
/// การปล่อยแค่ `VK_MENU`/`VK_CONTROL` (ตัวรวม) ไม่ล้างสถานะของซ้าย/ขวา
/// → ปุ่มยังค้างอยู่จริง → ผู้ใช้พิมพ์ไม่ได้ (ทุกตัวอักษรกลายเป็น Ctrl+<ตัวอักษร>)
fn release_stuck_modifiers() {
    send(&[
        key(VK_LCONTROL, true),
        key(VK_RCONTROL, true),
        key(VK_CONTROL, true),
    ]);
    send(&[key(VK_LMENU, true), key(VK_RMENU, true), key(VK_MENU, true)]);
    CTRL_INJECTED.store(false, Ordering::SeqCst);
    CTRL_SUSPENDED.store(false, Ordering::SeqCst);
}

/// ปล่อย modifier ที่ค้างอยู่ทั้งหมด — ใช้ตอนผู้ใช้กดเมนู "รีเซ็ตปุ่มที่ค้าง"
/// หรือตอนปิดโปรแกรม (กันทิ้ง `Ctrl` ค้างไว้ในระบบ)
pub fn release_all() {
    let fx = STATE.with(|s| s.borrow_mut().reset());
    for e in fx {
        apply(e);
    }
    // เผื่อกรณี modifier ค้างจาก session ก่อนหน้า (ที่เราไม่ได้เป็นคน inject)
    release_stuck_modifiers();
    tracing::warn!("รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt) ให้อัตโนมัติแล้ว");
}

/// แจ้งว่าระบบเพิ่งได้รับอินพุตจากผู้ใช้จริง (เรียกจากฝั่ง hotkey)
///
/// ใช้เป็นหลักฐานว่า "ผู้ใช้กดอยู่จริง" — ถ้าหลังจากนี้ hook ไม่เห็นอะไรเลย
/// แปลว่า hook ถูก Windows ถอดไปแล้ว (เพราะ callback ช้าเกิน 300 ms)
pub fn note_input_seen() {
    LAST_INPUT_MS.store(now_ms(), Ordering::SeqCst);
}

/// เรียกเป็นระยะจาก timer ของ thread หลัก
///
/// ทำ 2 อย่าง:
/// 1. ตรวจว่า `keyup` ของ Alt หายไปหรือไม่ (ผ่าน heartbeat) → กู้สถานะ
/// 2. ตรวจว่ามี `Ctrl`/`Alt` ค้างอยู่ในระบบที่ไม่ใช่ผู้ใช้กด → ปล่อยให้
pub fn tick() {
    // เขียน trace ที่สะสมไว้ (นอก hot path ของ hook) → ไม่ทำให้อินพุตช้า
    flush_trace();

    let was_suspect = STATE.with(|s| s.borrow().is_alt_suspect());
    let fx = STATE.with(|s| s.borrow_mut().tick(now_ms()));
    let now_suspect = STATE.with(|s| s.borrow().is_alt_suspect());
    if !was_suspect && now_suspect {
        tracing::warn!(
            "พบ keyup ของ Alt หายไป — ปล่อย Ctrl และกู้สถานะให้อัตโนมัติ (ตอนนี้พิมพ์ได้ตามปกติ)"
        );
    }
    for e in fx {
        apply(e);
    }

    let enabled = STATE.with(|s| s.borrow().enabled);
    if !enabled {
        return;
    }

    let (phys_ctrl, phys_alt) = STATE.with(|s| {
        let st = s.borrow();
        (st.phys_ctrl_down, st.is_phys_alt_down())
    });
    // ใช้ธง global (สถานะจริงในระบบ) ไม่ใช่ธงใน state machine
    // เพราะ worker thread อาจกด/ปล่อย Ctrl นอก state machine (suspend/resume ตอน Alt+Tab)
    let ctrl_injected = CTRL_INJECTED.load(Ordering::SeqCst);

    // ── ตรวจว่า hook ยังมีชีวิตอยู่ไหม ────────────────────────────────
    // Windows ถอด low-level hook ทิ้งเอง (โดยไม่แจ้ง) เมื่อ callback ช้าเกิน
    // LowLevelHooksTimeout (300 ms) → โหมด Mac จะ "ตาย" เงียบ ๆ จนกว่าจะเปิดโปรแกรมใหม่
    // ตรวจได้โดยเทียบเวลาที่ระบบเห็นอินพุต กับเวลาที่ hook เห็นเหตุการณ์
    let hook_ms = LAST_HOOK_EVENT_MS.load(Ordering::SeqCst);
    let input_ms = LAST_INPUT_MS.load(Ordering::SeqCst);
    if hook_is_dead(input_ms, hook_ms, HOOK_DEAD_MS) {
        tracing::warn!(
            "hook ไม่ตอบสนอง (ระบบเห็นอินพุตแต่ hook ไม่เห็น) — ติดตั้ง hook ใหม่"
        );
        install();
    }

    // เรา inject Ctrl ไว้ แต่ระบบไม่เห็นว่ากดอยู่ → flag เก่า (ค้างคิว) → ล้าง
    if should_reset_state(ctrl_is_down(), ctrl_injected) {
        STATE.with(|s| s.borrow_mut().reset());
    }

    // ระบบเห็น Ctrl ค้าง แต่ไม่ใช่ผู้ใช้กด และไม่ใช่ของเรา → ค้างจาก session ก่อนหน้า
    // (เช่น โปรแกรมถูกปิดตอนผู้ใช้กด ⌘ ค้างไว้)
    if should_release_stuck_ctrl(ctrl_is_down(), phys_ctrl, ctrl_injected) {
        if !WARNED_STUCK_CTRL.swap(true, Ordering::SeqCst) {
            tracing::warn!("พบ Ctrl ค้างในระบบ (ไม่ใช่ผู้ใช้กด) — ปล่อยให้อัตโนมัติ");
        }
        send(&[key(VK_LCONTROL, true), key(VK_RCONTROL, true), key(VK_CONTROL, true)]);
    } else {
        WARNED_STUCK_CTRL.store(false, Ordering::SeqCst);
    }

    // Alt ค้างในระบบ (เช่น inject Alt+Tab แล้วโปรแกรมตายก่อนปล่อย)
    // และผู้ใช้ไม่ได้กด Alt อยู่จริง → ปล่อยให้
    //
    // ดูทั้ง Alt ซ้าย/ขวา (ไม่ใช่ generic) เพราะ generic ค้างได้จากการ inject
    // แล้วถ้าเชื่อมันจะไปปล่อย Alt ของผู้ใช้ทิ้งเปล่า ๆ
    //
    // เทียบกับ `phys_alt` (ที่ผู้ใช้กดจริง) — ถ้าผู้ใช้กด Alt ขวา (AltGr) อยู่
    // ต้องไม่ไปปล่อยทิ้ง ไม่งั้นพิมพ์ไม่ได้
    if alt_side_down() && !phys_alt && !crate::win::inject::alt_injecting() {
        if !WARNED_STUCK_ALT.swap(true, Ordering::SeqCst) {
            tracing::warn!("พบ Alt ค้างในระบบ (ไม่ใช่ผู้ใช้กด) — ปล่อยให้อัตโนมัติ");
        }
        send(&[key(VK_LMENU, true), key(VK_RMENU, true)]);
    } else {
        WARNED_STUCK_ALT.store(false, Ordering::SeqCst);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code != HC_ACTION as i32 {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;
        let flags: KBDLLHOOKSTRUCT_FLAGS = kb.flags;
        let msg = wparam.0 as u32;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;

        // ── 1. อินพุตที่ถูก inject เอง ────────────────────────────────
        // อินพุตที่ MacKeys inject เอง (มี marker) → ต้องไม่ให้ระบบสับสน
        if kb.dwExtraInfo == INJECT_MARKER || kb.dwExtraInfo == INJECT_PASS_MARKER {
            if tracing::enabled!(tracing::Level::TRACE) {
                trace_push(format!("hook INJECTED vk={vk} down={down}"));
            }
            if kb.dwExtraInfo == INJECT_PASS_MARKER {
                // ต้องให้ระบบเห็นจริง (task switcher) → ปล่อยผ่าน แต่ไม่ให้แปลงซ้ำ
                return CallNextHookEx(None, code, wparam, lparam);
            }
            if vk == VK_LMENU {
                // ปล่อย Alt ที่เราสร้างเอง → กลืน ไม่ให้แอปเห็น Alt ที่หลอมขึ้นมา
                return LRESULT(1);
            }
            // Ctrl ที่เรา inject → ปล่อยผ่านให้แอปเห็นตามปกติ
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // อินพุตที่โปรแกรมอื่น inject → ไม่แตะ (กันไม่ให้แอปอื่นโดนผลของโหมด Mac)
        // ยกเว้นโหมดทดสอบที่เปิดด้วย MACKEYS_TEST_INJECTED=1
        if flags.contains(LLKHF_INJECTED) && !test_injected() {
            if tracing::enabled!(tracing::Level::TRACE) {
                // trace ไว้ด้วย เพราะถ้าอินพุตของผู้ใช้ถูก flag ว่า injected
                // (เช่นโปรแกรมอื่น inject หรือคีย์บอร์ดเสมือน) → โหมด Mac จะ "ไม่มีผล"
                // โดยที่ผู้ใช้ไม่รู้ตัว
                trace_push(format!("hook SKIP injected vk={vk} down={down}"));
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // ── 1b. จดสถานะปุ่ม modifier ที่ผู้ใช้กดจริง ─────────────────────
        // ต้องทำแม้ตอนโหมด Mac ปิดอยู่ เพราะ watchdog ใช้ข้อมูลนี้ตัดสินว่า
        // "ปุ่มค้างในระบบ" ควรปล่อยให้หรือเป็นแค่ผู้ใช้กดค้างอยู่เอง
        STATE.with(|s| s.borrow_mut().note_physical(vk, down));

        let enabled = STATE.with(|s| s.borrow().enabled);
        if !enabled {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // ── 2. ส่งเหตุการณ์เข้า state machine แล้วทำตาม effect ที่ได้ ──
        let ev = if down {
            Event::KeyDown(vk)
        } else {
            Event::KeyUp(vk)
        };
        // จำค่า "Space ในโหมด ⌘" ไว้ก่อนประมวลผล เพราะ keyup จะเป็นตัวล้างธงนี้
        let mac_space_before = STATE.with(|s| s.borrow().is_mac_space());
        let t0 = std::time::Instant::now();
        LAST_HOOK_EVENT_MS.store(now_ms(), Ordering::SeqCst);
        let fx = STATE.with(|s| s.borrow_mut().handle(ev, now_ms()));
        for e in fx {
            apply(e);
        }

        // บันทึก trace ลงคิว (ไม่เขียนไฟล์ตรงนี้ — ดู `trace_push`)
        // วัดเวลาที่ hook ใช้ด้วย ถ้าเกิน ~1 ms แปลว่าเราเป็นตัวที่ทำให้อินพุตช้า
        if tracing::enabled!(tracing::Level::TRACE) {
            let alt = STATE.with(|s| s.borrow().is_alt_down());
            let ctrl = STATE.with(|s| s.borrow().is_ctrl_injected());
            let held = STATE.with(|s| s.borrow().held_count());
            let us = t0.elapsed().as_micros();
            trace_push(format!(
                "hook vk={vk} down={down} alt={alt} ctrl={ctrl} held={held} took={us}us"
            ));
        }

        // ── 3. ปุ่ม Alt ต้องถูกกลืนเสมอ (ทั้ง down และ up) ─────────────
        if vk == VK_LMENU {
            return LRESULT(1);
        }

        // ── 4. ปุ่ม Tab ตอนอยู่ในโหมด ⌘ ต้องถูกกลืนเช่นกัน ────────────
        // เรา inject `Tab` เองไปยัง task switcher → ถ้าไม่กลืน Tab จริง
        // ระบบจะเห็น 2 ครั้ง (Tab จริง + Tab ที่ inject) → เลื่อนข้าม 2 หน้าต่าง
        if vk == VK_TAB && STATE.with(|s| s.borrow().is_alt_down()) {
            return LRESULT(1);
        }

        // ── 5. ปุ่ม Space ในโหมด ⌘ ต้องถูกกลืน (เราแปลงเป็น Win+Space แล้ว) ──
        // ถ้าไม่กลืน → แอปจะได้ space ปกติ + ยังสลับภาษาด้วย (ผิดทั้งสองอย่าง)
        if vk == VK_SPACE {
            let swallow = if down {
                STATE.with(|s| s.borrow().is_mac_space())
            } else {
                mac_space_before
            };
            if swallow {
                return LRESULT(1);
            }
        }

        CallNextHookEx(None, code, wparam, lparam)
    }
}

/// ติดตั้ง hook — ต้องเรียกจาก thread ที่มี message loop
///
/// **ติดตั้งใหม่ทุกครั้ง** (ถอดของเดิมออกก่อน) เพราะ Windows อาจถอด low-level hook
/// ทิ้งเองโดยไม่แจ้ง เมื่อ callback ช้าเกิน `LowLevelHooksTimeout` (300 ms)
/// ถ้าเราเชื่อว่าตัวเองติดตั้งอยู่แล้ว โหมด Mac จะตายเงียบ ๆ
/// (อาการ: กด ⌘+อะไรก็ไม่มีผล แต่ hotkey อื่นยังทำงาน)
pub fn install() -> bool {
    let old = HOOK.with(|h| h.borrow_mut().take());
    if let Some(h) = old {
        unsafe {
            let _ = UnhookWindowsHookEx(h);
        }
    }
    unsafe {
        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
            Ok(h) => {
                HOOK.with(|slot| *slot.borrow_mut() = Some(h));
                LAST_HOOK_EVENT_MS.store(now_ms(), Ordering::SeqCst);
                true
            }
            Err(e) => {
                tracing::error!("ติดตั้ง keyboard hook ไม่สำเร็จ: {e}");
                false
            }
        }
    }
}

/// ถอด hook ออก
pub fn uninstall() {
    let hook = HOOK.with(|h| h.borrow_mut().take());
    if let Some(h) = hook {
        unsafe {
            let _ = UnhookWindowsHookEx(h);
        }
    }
    // ปล่อย Ctrl ที่ค้างอยู่ก่อนจบโปรแกรม — ไม่งั้นผู้ใช้จะพิมพ์ไม่ได้
    // (เกิดได้เมื่อโปรแกรมถูกปิด/ตายขณะผู้ใช้กด ⌘ ค้างอยู่)
    release_all();
}

/// เปิด/ปิดโหมด Mac (Alt ทำหน้าที่เป็น Ctrl)
pub fn set_enabled(on: bool) {
    if on {
        install();
    } else {
        // ปิดโหมด Mac → ต้องปล่อย Ctrl ที่ค้างอยู่ด้วย ไม่งั้นผู้ใช้จะพิมพ์ไม่ได้
        release_all();
    }
    STATE.with(|s| s.borrow_mut().enabled = on);
    tracing::info!("โหมด Mac (Alt = ⌘): {}", if on { "เปิด" } else { "ปิด" });
}

/// กันไม่ให้ compiler ฟ้องเรื่อง ptr ที่ไม่ได้ใช้
#[allow(dead_code)]
fn _unused(_p: *const u8) {
    let _ = ptr::null::<u8>();
}

#[cfg(test)]
mod tests {
    use super::{hook_is_dead, parse_test_flag, should_release_stuck_ctrl, should_reset_state};

    /// `MACKEYS_TEST_INJECTED` ต้องอ่านค่าจริง ไม่ใช่แค่ดูว่าตัวแปรมีอยู่
    /// เคยพลาดเพราะใช้ `var_os(...).is_some()` → ตั้ง `=0` ก็ยังเปิดโหมดทดสอบ
    #[test]
    fn test_flag_reads_actual_value() {
        // ไม่ได้ตั้ง → ปิด
        assert!(!parse_test_flag(None));
        // ค่าที่แปลว่า "ปิด"
        for v in ["", "0", "false", "FALSE", "no", "off", "  0  "] {
            assert!(
                !parse_test_flag(Some(v)),
                "ค่า {v:?} ต้องถือว่าปิดโหมดทดสอบ"
            );
        }
        // ค่าที่แปลว่า "เปิด"
        for v in ["1", "true", "TRUE", "yes", "on", " 1 "] {
            assert!(
                parse_test_flag(Some(v)),
                "ค่า {v:?} ต้องถือว่าเปิดโหมดทดสอบ"
            );
        }
    }

    /// ตรวจว่า hook ตายได้จากหลักฐานเดียว: ระบบเห็นอินพุต แต่ hook ไม่เห็น
    #[test]
    fn detects_dead_hook_from_input_without_events() {
        const DEAD: u64 = 2_000;
        // ยังไม่เคยเห็นอินพุต → ยังพิสูจน์ไม่ได้ ต้องไม่ติดตั้งใหม่มือเปล่า
        assert!(!hook_is_dead(0, 0, DEAD));

        // ระบบเห็นอินพุต แต่ hook ไม่เห็นเลยเกิน 2 วินาที → ตาย
        assert!(hook_is_dead(10_000, 7_000, DEAD));
        // ... เกินเพียงเล็กน้อย → ยังไม่ตาย
        assert!(!hook_is_dead(10_000, 9_500, DEAD));
        assert!(!hook_is_dead(10_000, 10_000, DEAD));

        // เวลาของ hook ใหม่กว่าอินพุต (เพิ่งซ่อม) → ปกติ
        assert!(!hook_is_dead(5_000, 9_000, DEAD));
    }

    /// watchdog ต้องปล่อย Ctrl เฉพาะตอนที่ "ไม่มีใครเป็นเจ้าของ" เท่านั้น
    #[test]
    fn stuck_ctrl_release_only_when_nobody_owns_it() {
        // ระบบเห็น Ctrl ค้าง แต่ไม่มีทั้งผู้ใช้กด และไม่ใช่ของเรา → ต้องปล่อย
        assert!(should_release_stuck_ctrl(true, false, false));
        // ผู้ใช้กด Ctrl อยู่ → ห้ามปล่อย (จะไปตัดมือผู้ใช้)
        assert!(!should_release_stuck_ctrl(true, true, false));
        // เราเป็นคนกดอยู่เอง → ห้ามปล่อย
        assert!(!should_release_stuck_ctrl(true, false, true));
        // ไม่มี Ctrl ค้าง → ไม่ต้องทำอะไร
        assert!(!should_release_stuck_ctrl(false, false, false));
    }

    /// ตอน `suspend_ctrl()` (ก่อนส่ง Alt+Tab) ระบบจะไม่เห็น Ctrl ชั่วคราว
    /// แต่ธง "เรา inject อยู่" ถูกเคลียร์ไปแล้ว → ต้องไม่ล้าง state machine
    /// ไม่งั้นโหมด Mac จะลืมว่าผู้ใช้กด ⌘ อยู่ (กดต่อแล้วไม่เป็น Ctrl+<ปุ่ม>)
    #[test]
    fn state_is_not_reset_while_ctrl_is_suspended() {
        // กำลังพัก Ctrl (ปล่อยออกชั่วคราว) → ระบบไม่เห็น Ctrl และเราไม่ได้ถือว่า inject อยู่
        // → ต้องไม่รีเซ็ต state
        assert!(!should_reset_state(false, false));

        // เรา inject Ctrl ไว้ แต่ระบบไม่เห็นเลยนาน ๆ → flag เก่า → ล้าง
        assert!(should_reset_state(false, true));

        // ระบบเห็น Ctrl ที่เรา inject → ปกติ
        assert!(!should_reset_state(true, true));
    }
}
