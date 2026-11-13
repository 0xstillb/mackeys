use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_INPUTLANGCHANGEREQUEST};

/// ข้อมูล keyboard layout หนึ่งรายการ
#[derive(Debug, Clone)]
pub struct LayoutInfo {
    pub hkl: HKL,
    /// รหัสภาษา เช่น 0x0409 (en-US), 0x041D (th-TH)
    pub lang_id: u32,
    pub name: String,
}

/// รายการ layout ทั้งหมดที่ติดตั้งในเครื่อง
pub fn installed() -> Vec<LayoutInfo> {
    unsafe {
        let count = GetKeyboardLayoutList(None);
        if count <= 0 {
            return Vec::new();
        }
        let mut buf: Vec<HKL> = vec![HKL::default(); count as usize];
        let got = GetKeyboardLayoutList(Some(&mut buf));
        let got = got.clamp(0, count) as usize;
        buf.truncate(got);
        buf.into_iter()
            .map(|hkl| {
                let lang_id = hkl.0 as usize as u32 & 0xFFFF;
                LayoutInfo {
                    hkl,
                    lang_id,
                    name: lang_name(lang_id),
                }
            })
            .collect()
    }
}

/// layout ที่กำลังใช้อยู่ของหน้าต่างที่ active
pub fn current() -> Option<LayoutInfo> {
    unsafe {
        let tid = crate::win::util::foreground_thread_id();
        if tid == 0 {
            return None;
        }
        let hkl = GetKeyboardLayout(tid);
        if hkl.0.is_null() {
            return None;
        }
        let lang_id = hkl.0 as usize as u32 & 0xFFFF;
        Some(LayoutInfo {
            hkl,
            lang_id,
            name: lang_name(lang_id),
        })
    }
}

pub fn current_lang_id() -> u32 {
    current().map(|l| l.lang_id).unwrap_or(0)
}

/// สลับภาษาตามเป้าหมายที่ระบุ
pub fn switch(target: &str, cycle: &[String]) -> Result<String> {
    match target.trim().to_ascii_lowercase().as_str() {
        "next" => switch_next_installed(),
        "cycle" => switch_cycle(cycle),
        other => switch_to(other),
    }
}

/// สลับไป layout ถัดไปในรายการที่ติดตั้งจริง
fn switch_next_installed() -> Result<String> {
    let list = installed();
    if list.len() < 2 {
        bail!("ติดตั้ง keyboard layout ไว้ไม่ถึง 2 รายการ จึงสลับไม่ได้");
    }
    let cur = current_lang_id();
    let pos = list.iter().position(|l| l.lang_id == cur).unwrap_or(0);
    let next = &list[(pos + 1) % list.len()];
    activate(next.hkl, next.lang_id)?;
    Ok(next.name.clone())
}

/// สลับไปภาษาถัดไปตามรายการ `cycle` ใน config
fn switch_cycle(cycle: &[String]) -> Result<String> {
    if cycle.is_empty() {
        bail!("ยังไม่ได้ตั้งรายการภาษาใน [layouts].cycle");
    }
    let all = installed();
    let matched: Vec<LayoutInfo> = all
        .into_iter()
        .filter(|li| cycle.iter().any(|alias| alias_matches(alias, li.lang_id)))
        .collect();

    if matched.is_empty() {
        bail!(
            "ไม่พบ layout ตามรายการ {} — ดูได้จากคำสั่ง `mackeys list-layouts`",
            cycle.join(", ")
        );
    }

    let cur = current_lang_id();
    let pos = matched.iter().position(|li| li.lang_id == cur).unwrap_or(0);
    let next = &matched[(pos + 1) % matched.len()];
    activate(next.hkl, next.lang_id)?;
    Ok(next.name.clone())
}

/// สลับไปภาษาที่ระบุชื่อ เช่น "th-TH"
fn switch_to(alias: &str) -> Result<String> {
    let want = alias_lang_id(alias)
        .ok_or_else(|| anyhow!("ไม่รู้จักภาษา \"{alias}\" (ลองใช้ en-US, th-TH หรือรหัส hex เช่น 041d)"))?;
    let all = installed();
    // หาแบบตรงรหัสก่อน ถ้าไม่เจอลองเทียบรหัสภาษาหลัก (เช่น ขอ th-TH แต่เครื่องมี 041D)
    let found = all
        .iter()
        .find(|li| li.lang_id == want)
        .or_else(|| all.iter().find(|li| (li.lang_id >> 10) == (want >> 10)))
        .ok_or_else(|| anyhow!("ไม่พบ layout \"{alias}\" ในเครื่องนี้"))?;
    activate(found.hkl, found.lang_id)?;
    Ok(found.name.clone())
}

/// สั่งเปลี่ยนภาษาอินพุต
///
/// ใช้ `Win+Space` ให้ **Windows สลับเอง** เหมือนที่ผู้ใช้กดเอง
/// → ทำงานได้ทุกที่ (ไม่ต้องอยู่ในช่องพิมพ์) เพราะเป็นกลไกของระบบ
/// (เมธอดเดิมใช้ `WM_INPUTLANGCHANGEREQUEST` ซึ่งส่งให้ "หน้าต่างที่โฟกัส" เท่านั้น
///  ถ้าหน้าต่างนั้นไม่รับข้อความนี้ (Explorer, เกม, แอปที่วาดเอง) ก็จะไม่มีอะไรเกิดขึ้น)
fn activate(hkl: HKL, target_lang: u32) -> Result<()> {
    // จำนวนครั้งที่ต้องกด Win+Space เพื่อไปถึงภาษาเป้าหมาย
    let order: Vec<u32> = installed().iter().map(|l| l.lang_id).collect();
    let current = current_lang_id();
    let presses = presses_to_reach(&order, current, target_lang);

    if presses > 0 {
        // ถ้าโหมด Mac inject Ctrl ค้างอยู่ → ปล่อยออกก่อน ไม่งั้นจะกลายเป็น Ctrl+Win+Space
        let suspended = crate::win::hook::suspend_ctrl();
        for _ in 0..presses {
            // ใช้ตัวที่ "ไม่แตะ modifier" เพราะเราปล่อย Ctrl ไว้ทั้งลูปอยู่แล้ว
            crate::win::inject::win_space_preserving_modifiers();
            std::thread::sleep(Duration::from_millis(180));
            if current_lang_id() == target_lang {
                break;
            }
        }
        if suspended {
            crate::win::hook::resume_ctrl();
        }
        if current_lang_id() == target_lang {
            return Ok(());
        }
    }

    // ทางสำรอง: ส่ง WM_INPUTLANGCHANGEREQUEST ให้หน้าต่างที่ focus อยู่
    // (แอปที่จัดการข้อความนี้เองจะเปลี่ยนได้ทันที โดยไม่ต้องรอ Win+Space)
    unsafe {
        let target = crate::win::util::focused_window();
        if target.0.is_null() {
            bail!("ไม่พบหน้าต่างที่ active");
        }
        match PostMessageW(
            Some(target),
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(hkl.0 as usize as isize),
        ) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!("ส่ง WM_INPUTLANGCHANGEREQUEST ไม่สำเร็จ ({e}) — ใช้ Win+Space แทน");
                crate::win::inject::win_space();
                Ok(())
            }
        }
    }
}

/// จำนวนครั้งที่ต้องกด `Win+Space` เพื่อไปถึงภาษาเป้าหมาย (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// `Win+Space` วนภาษาตามลำดับที่ Windows ติดตั้งไว้ (`GetKeyboardLayoutList`)
/// → ต้องนับว่าอยู่ตรงไหนแล้วถึงจะรู้ว่าต้องกดกี่ครั้ง
///
/// คืน 0 = อยู่ที่เป้าหมายอยู่แล้ว (ไม่ต้องกด)
fn presses_to_reach(order: &[u32], current: u32, target: u32) -> usize {
    if current == target {
        return 0;
    }
    let (Some(i_cur), Some(i_tgt)) = (
        order.iter().position(|l| *l == current),
        order.iter().position(|l| *l == target),
    ) else {
        // หาไม่เจอ → เดาว่า 1 ครั้ง (ให้ Windows ตัดสินใจเอง)
        return 1;
    };
    let n = order.len();
    if n == 0 {
        return 1;
    }
    (i_tgt + n - i_cur) % n
}

/// แปลงชื่อภาษา/รหัส → language id
pub fn alias_lang_id(alias: &str) -> Option<u32> {
    let a = alias.trim().to_ascii_lowercase();
    Some(match a.as_str() {
        "en" | "en-us" | "eng" | "us" => 0x0409,
        "th" | "th-th" | "tha" | "thai" => 0x041E,
        _ => {
            let hex = a.strip_prefix("0x").unwrap_or(&a);
            if hex.len() == 4 || hex.len() == 8 {
                if let Ok(v) = u32::from_str_radix(hex, 16) {
                    return Some(v & 0xFFFF);
                }
            }
            return None;
        }
    })
}

/// ชื่อที่ตั้งไว้ตรงกับ layout นี้ไหม (เทียบรหัสภาษาหลักด้วย เช่น 041E กับ 041D คือภาษาไทยเหมือนกัน)
fn alias_matches(alias: &str, lang_id: u32) -> bool {
    let Some(want) = alias_lang_id(alias) else {
        return false;
    };
    want == lang_id || (want >> 10) == (lang_id >> 10)
}

/// ตั้งชื่อที่อ่านง่ายให้ language id
pub fn lang_name(lang_id: u32) -> String {
    match lang_id {
        0x0409 => "en-US".into(),
        0x041D | 0x041E => "th-TH".into(),
        0x0419 => "ru-RU".into(),
        0x0411 => "ja-JP".into(),
        0x0804 => "zh-CN".into(),
        0x0C0A => "es-ES".into(),
        other => format!("{other:04X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::presses_to_reach;

    /// จำนวนครั้งที่ต้องกด Win+Space เพื่อไปถึงภาษาเป้าหมาย
    #[test]
    fn presses_to_reach_counts_system_order() {
        let order = [0x0409u32, 0x041E]; // en-US, th-TH
        // อยู่ที่เป้าหมายแล้ว → ไม่ต้องกด
        assert_eq!(presses_to_reach(&order, 0x0409, 0x0409), 0);
        // ไปข้างหน้า 1 ครั้ง
        assert_eq!(presses_to_reach(&order, 0x0409, 0x041E), 1);
        // ไปข้างหลัง 1 ครั้ง (วนกลับ)
        assert_eq!(presses_to_reach(&order, 0x041E, 0x0409), 1);
    }

    /// 3 ภาษา: ไปข้างหน้าได้ 2 ครั้ง
    #[test]
    fn presses_to_reach_wraps_around() {
        let order = [0x0409u32, 0x041E, 0x0419]; // en-US, th-TH, ru-RU
        assert_eq!(presses_to_reach(&order, 0x0409, 0x0419), 2);
        assert_eq!(presses_to_reach(&order, 0x0419, 0x0409), 1);
        assert_eq!(presses_to_reach(&order, 0x0419, 0x041E), 2);
    }

    /// ภาษาที่ไม่มีในเครื่อง → ให้ Windows ตัดสินใจเอง 1 ครั้ง
    #[test]
    fn presses_to_reach_falls_back_when_unknown() {
        assert_eq!(presses_to_reach(&[0x0409u32, 0x041E], 0x0409, 0x1234), 1);
        assert_eq!(presses_to_reach(&[], 0x0409, 0x041E), 1);
    }
}
