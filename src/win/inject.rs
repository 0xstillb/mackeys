use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    SendInput, VIRTUAL_KEY,
};

use crate::hotkey::Hotkey;

/// กำลัง inject `Alt` อยู่ตอนนี้หรือไม่
///
/// watchdog ของ hook ต้องรู้เรื่องนี้ เพราะตอนที่เรากด `Alt` เอง (เช่นตอนส่ง `Alt+Tab`)
/// ระบบจะเห็นว่า Alt ถูกกดอยู่ ทั้งที่ผู้ใช้ไม่ได้กด → ถ้าไม่กันไว้
/// watchdog จะไป "ปล่อย Alt" ทิ้งกลางคัน → task switcher ของ Windows ถูกปิดก่อนเวลา
static ALT_INJECTING: AtomicBool = AtomicBool::new(false);

/// ตอนนี้เรากำลัง inject `Alt` อยู่หรือไม่ (ให้ watchdog ใน `hook.rs` รู้)
pub fn alt_injecting() -> bool {
    ALT_INJECTING.load(Ordering::SeqCst)
}

const VK_LWIN: u16 = 0x5B;
const VK_SHIFT: u16 = 0x10;
const VK_SPACE: u16 = 0x20;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;
const VK_TAB: u16 = 0x09;
const VK_S: u16 = 0x53;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_CONTROL: u16 = 0x11;

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                // ต้องใส่ marker "ปล่อยผ่าน" ไว้
                //
                // ถ้าไม่ใส่ marker เลย hook จะมองว่าเป็น "อินพุตที่โปรแกรมอื่น inject"
                // → ในโหมดทดสอบ (`MACKEYS_TEST_INJECTED=1`) hook จะเอาไปแปลงเป็น ⌘
                //   ทำให้ `Alt+F4` กลายเป็น `Ctrl+Alt+F4` → แอปไม่ปิด
                // → ด้วย marker hook จะรู้ว่าเป็นของเรา แล้วปล่อยผ่านให้แอปเห็นจริง
                dwExtraInfo: crate::win::hook::INJECT_PASS_MARKER,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// กดปุ่มตามลำดับแล้วปล่อยกลับกัน
pub fn send_combo(keys: &[u16]) {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(keys.len() * 2);
    for &k in keys {
        inputs.push(key(k, false));
    }
    for &k in keys.iter().rev() {
        inputs.push(key(k, true));
    }
    send(&inputs);
}

/// คอมโบนี้ต้องการ modifier "สะอาด" ไหม (ฟังก์ชันบริสทึก — เทสต์ได้)
///
/// ในโหมด Mac เราอาจ inject `Ctrl` ค้างไว้ (ตอนผู้ใช้กด ⌘ อยู่)
/// ถ้ายังกดอยู่ตอนส่งคอมโบที่ไม่มี `Ctrl` อยู่ในตัวเอง
/// ระบบจะเห็นเป็น `Ctrl+<คอมโบ>` → **คอมโบนั้นไม่ทำงาน**
/// (เคยเป็นบั๊ก `⌘+Shift+4` ไม่ทำงาน: เห็นเป็น `Ctrl+Win+Shift+S` → Snipping Tool ไม่เปิด)
pub fn needs_clean_modifiers(keys: &[u16]) -> bool {
    !keys
        .iter()
        .any(|&k| k == VK_LCONTROL || k == VK_RCONTROL || k == VK_CONTROL)
}

/// ส่งคอมโบโดยปล่อย `Ctrl` ที่โหมด Mac inject ค้างอยู่ก่อน แล้วกดคืนหลัง
///
/// คืนค่า `true` ถ้าต้องการ modifier สะอาด (ไม่มี Ctrl ในตัวคอมโบ)
pub fn send_clean_combo(keys: &[u16]) {
    if !needs_clean_modifiers(keys) {
        send_combo(keys);
        return;
    }
    let suspended = crate::win::hook::suspend_ctrl();
    send_combo(keys);
    if suspended {
        crate::win::hook::resume_ctrl();
    }
}

/// Win+Space — ใช้เป็นทางเลือกสำรองตอนสลับภาษา
pub fn win_space() {
    send_clean_combo(&[VK_LWIN, VK_SPACE]);
}

/// Win+Space โดย **ไม่**แตะ modifier ที่ถูกปล่อยไว้แล้ว
///
/// ใช้ในลูปที่กดหลายครั้งติด ๆ กัน (ผู้เรียกจัดการเรื่องปล่อย Ctrl เองแล้ว)
pub fn win_space_preserving_modifiers() {
    send_combo(&[VK_LWIN, VK_SPACE]);
}

/// Win+Shift+S — เปิด Snipping Tool แบบเลือกบริเวณ
pub fn win_shift_s() {
    send_clean_combo(&[VK_LWIN, VK_SHIFT, VK_S]);
}

/// Alt+Tab — สลับไปหน้าต่างถัดไป
///
/// ใช้ `VK_LMENU` (Alt ซ้าย) แทน `VK_MENU` (ตัวรวม) เพราะการ inject ด้วยตัวรวม
/// ทำให้สถานะ Alt ในระบบค้าง และ task switcher ของ Windows ค้างไว้โดยไม่ปิด
pub fn alt_tab() {
    // ต้องกด Alt ค้างไว้สักครู่ ให้ Windows เปิด task switcher แล้วค่อยปล่อย
    ALT_INJECTING.store(true, Ordering::SeqCst);
    send(&[key(VK_LMENU, false), key(VK_TAB, false), key(VK_TAB, true)]);
    std::thread::sleep(Duration::from_millis(80));
    send(&[key(VK_LMENU, true)]);
    ALT_INJECTING.store(false, Ordering::SeqCst);
}

/// ตอนนี้ปุ่ม Alt ถูกกดอยู่จริงหรือเปล่า (ดูเฉพาะซ้าย/ขวา — ตัวรวมอาจค้างจากการ inject)
fn alt_side_down() -> bool {
    unsafe {
        [VK_LMENU, VK_RMENU]
            .iter()
            .any(|&vk| (GetAsyncKeyState(vk as i32) as i32) & 0x8000 != 0)
    }
}

/// ส่งชุดคีย์ตามที่ระบุ ("ctrl+c", "alt+f4", "win+h") ไปยังหน้าต่างที่ focus อยู่
///
/// ถ้าปุ่ม Alt ถูกกดอยู่จริงตอนส่ง จะปล่อย Alt ออกก่อน แล้วกดกลับหลังเสร็จ
/// เพื่อไม่ให้แอปที่รับได้รับ "Ctrl+Alt+X" แทน "Ctrl+X"
pub fn send_hotkey(hk: &Hotkey) {
    let mut keys: Vec<u16> = Vec::with_capacity(5);
    // ใช้ปุ่ม "ฝั่ง" (ซ้าย) แทนตัวรวม เพราะการ inject ด้วย `VK_CONTROL`/`VK_MENU` (ตัวรวม)
    // ทำให้สถานะฝั่งซ้าย/ขวาในระบบไม่ถูกล้าง → ปุ่มค้าง (เคยเป็นบั๊ก "พิมพ์ไม่ได้")
    if hk.mods.ctrl {
        keys.push(VK_LCONTROL);
    }
    if hk.mods.alt {
        keys.push(VK_LMENU);
    }
    if hk.mods.shift {
        keys.push(VK_SHIFT);
    }
    if hk.mods.win {
        keys.push(VK_LWIN);
    }
    keys.push(hk.vk as u16);

    // กันไม่ให้คีย์ที่เรา inject ไปโดน hotkey ที่ลงทะเบียนไว้ (วนลูปได้)
    crate::engine::suppress_hotkeys();

    // **ปล่อย `Ctrl` ที่โหมด Mac inject ค้างอยู่ก่อน**
    //
    // ตอนผู้ใช้กด ⌘+Q โหมด Mac จะ inject `Ctrl` ไว้ (เพื่อให้ hotkey ctrl+q ยิง)
    // ถ้ายังกด Ctrl อยู่ตอนส่ง `Alt+F4` → ระบบจะเห็นเป็น `Ctrl+Alt+F4` → แอปไม่ปิด
    let suspended = crate::win::hook::suspend_ctrl();

    let alt_down = alt_side_down();
    if alt_down {
        // ปล่อย Alt ทั้งซ้ายและขวา (อาจค้างจาก session ก่อนหน้า)
        ALT_INJECTING.store(true, Ordering::SeqCst);
        send(&[key(VK_LMENU, true), key(VK_RMENU, true)]);
        std::thread::sleep(Duration::from_millis(20));
    }

    send_combo(&keys);

    if alt_down {
        std::thread::sleep(Duration::from_millis(20));
        // กด Alt กลับคืน — ผู้ใช้ปล่อยเองได้ตอนยกนิ้ว
        // (ถ้าไม่กดกลับ ผู้ใช้จะเสียการกด Alt ค้างไว้)
        send(&[key(VK_LMENU, false), key(VK_RMENU, false)]);
        ALT_INJECTING.store(false, Ordering::SeqCst);
    }

    // กด Ctrl กลับคืน (เฉพาะกรณีที่ผู้ใช้ยังกด ⌘ อยู่)
    if suspended {
        crate::win::hook::resume_ctrl();
    }

    std::thread::sleep(Duration::from_millis(120));
    crate::engine::resume_hotkeys();
}

#[cfg(test)]
mod tests {
    use super::{key, needs_clean_modifiers};
    use crate::win::hook::INJECT_PASS_MARKER;

    /// คีย์ที่ MacKeys inject เอง ต้องมี marker "ปล่อยผ่าน" เสมอ
    ///
    /// ถ้าไม่มี marker → hook จะมองว่าเป็นอินพุตของโปรแกรมอื่น
    /// → ในโหมดทดสอบ (`MACKEYS_TEST_INJECTED=1`) จะถูกแปลงเป็น ⌘
    /// → `Alt+F4` กลายเป็น `Ctrl+Alt+F4` → แอปไม่ปิด (บั๊ก ⌘Q)
    #[test]
    fn injected_keys_carry_pass_marker() {
        let down = key(0x41, false);
        let up = key(0x41, true);

        assert_eq!(
            unsafe { down.Anonymous.ki.dwExtraInfo } as usize,
            INJECT_PASS_MARKER,
            "คีย์ที่ inject ต้องมี marker ปล่อยผ่าน"
        );
        assert_eq!(
            unsafe { up.Anonymous.ki.dwExtraInfo } as usize,
            INJECT_PASS_MARKER,
            "คีย์ที่ปล่อยก็ต้องมี marker เหมือนกัน"
        );
    }

    /// คอมโบที่ไม่มี `Ctrl` ต้องปล่อย Ctrl ที่โหมด Mac inject ค้างไว้ก่อนเสมอ
    ///
    /// ไม่งั้นระบบจะเห็นเป็น `Ctrl+<คอมโบ>` → คอมโบนั้นไม่ทำงาน
    /// (บั๊กจริง: `⌘+Shift+4` → เห็นเป็น `Ctrl+Win+Shift+S` → Snipping Tool ไม่เปิด)
    #[test]
    fn combos_without_ctrl_need_clean_modifiers() {
        // Win+Shift+S (จับภาพบริเวณ) → ต้องปล่อย Ctrl ที่ค้าง
        assert!(needs_clean_modifiers(&[0x5B, 0x10, 0x53]));
        // Win+Space (สลับภาษา) → ต้องปล่อย Ctrl ที่ค้าง
        assert!(needs_clean_modifiers(&[0x5B, 0x20]));

        // คอมโบที่มี Ctrl อยู่แล้ว → ไม่ต้องปล่อย (Ctrl ที่ inject ค้างคือตัวเดียวกัน)
        assert!(!needs_clean_modifiers(&[0xA2, 0x43])); // Ctrl+C
        assert!(!needs_clean_modifiers(&[0xA3, 0x43])); // Ctrl+C (ขวา)
        assert!(!needs_clean_modifiers(&[0x11, 0x43])); // Ctrl (ตัวรวม) + C

        // Alt+S (⌘` → alt+`) → ไม่มี Ctrl ในคอมโบ → ต้องปล่อย Ctrl ที่ค้างเช่นกัน
        assert!(needs_clean_modifiers(&[0xA4, 0x73]));
    }
}
