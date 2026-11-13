use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Mods {
    fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.win
    }
}

/// คีย์ลัดแบบ "ctrl+shift+3"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub mods: Mods,
    pub vk: u32,
}

impl Hotkey {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut mods = Mods::default();
        let mut vk: Option<u32> = None;

        for part in raw.split('+') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "lctrl" | "rctrl" => mods.ctrl = true,
                "alt" | "lalt" | "ralt" | "option" => mods.alt = true,
                "shift" | "lshift" | "rshift" => mods.shift = true,
                "win" | "windows" | "lwin" | "rwin" | "super" | "cmd" | "meta" => mods.win = true,
                other => {
                    if vk.is_some() {
                        return Err(format!("คีย์ลัด \"{raw}\" มีปุ่มหลักมากกว่า 1 ปุ่ม"));
                    }
                    vk = Some(key_vk(other).ok_or_else(|| format!("ไม่รู้จักปุ่ม \"{other}\""))?);
                }
            }
        }

        let vk = vk.ok_or_else(|| format!("คีย์ลัด \"{raw}\" ไม่มีปุ่มหลัก"))?;
        if mods.is_empty() {
            return Err(format!("คีย์ลัด \"{raw}\" ต้องมีอย่างน้อยหนึ่งปุ่ม modifier"));
        }

        Ok(Self { mods, vk })
    }

    /// ตัวแปรแบบ RegisterHotKey (ไม่แยกซ้าย/ขวา — Win32 ไม่รองรับ)
    pub fn win_mods(&self) -> HOT_KEY_MODIFIERS {
        let mut v = MOD_NOREPEAT.0;
        if self.mods.ctrl {
            v |= MOD_CONTROL.0;
        }
        if self.mods.alt {
            v |= MOD_ALT.0;
        }
        if self.mods.shift {
            v |= MOD_SHIFT.0;
        }
        if self.mods.win {
            v |= MOD_WIN.0;
        }
        HOT_KEY_MODIFIERS(v)
    }

    /// ข้อความสำหรับแสดงผลใน GUI
    pub fn label(&self) -> String {
        let mut s = String::new();
        if self.mods.ctrl {
            s.push_str("Ctrl+");
        }
        if self.mods.alt {
            s.push_str("Alt+");
        }
        if self.mods.shift {
            s.push_str("Shift+");
        }
        if self.mods.win {
            s.push_str("Win+");
        }
        s.push_str(&vk_name(self.vk));
        s
    }
}

/// แปลงชื่อปุ่ม → virtual key code
pub fn key_vk(name: &str) -> Option<u32> {
    let n = name.trim().to_ascii_lowercase();
    if n.len() == 1 {
        let c = n.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Some(c.to_ascii_uppercase() as u32);
        }
        if c.is_ascii_digit() {
            return Some(c as u32);
        }
    }

    Some(match n.as_str() {
        "space" => 0x20,
        "tab" => 0x09,
        "enter" | "return" => 0x0D,
        "esc" | "escape" => 0x1B,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "insert" | "ins" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" | "pgup" => 0x21,
        "pagedown" | "pgdn" => 0x22,
        "up" => 0x26,
        "down" => 0x28,
        "left" => 0x25,
        "right" => 0x27,
        "printscreen" | "prtsc" | "print" => 0x2C,
        "pause" => 0x13,
        "capslock" => 0x14,
        "minus" | "-" => 0xBD,
        "equal" | "=" => 0xBB,
        "lbracket" | "[" => 0xDB,
        "rbracket" | "]" => 0xDD,
        "backslash" | "\\" => 0xDC,
        "semicolon" | ";" => 0xBA,
        "apostrophe" | "'" => 0xDE,
        "grave" | "`" => 0xC0,
        "comma" | "," => 0xBC,
        "dot" | "." => 0xBE,
        "slash" | "/" => 0xBF,
        _ => {
            if let Some(rest) = n.strip_prefix('f')
                && let Ok(num) = rest.parse::<u32>()
                && (1..=24).contains(&num)
            {
                return Some(0x6F + num);
            }
            return None;
        }
    })
}

/// แปลง virtual key code → ชื่อที่อ่านง่าย
pub fn vk_name(vk: u32) -> String {
    match vk {
        0x20 => "Space".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x1B => "Esc".into(),
        0x08 => "Backspace".into(),
        0x2E => "Delete".into(),
        0x2D => "Insert".into(),
        0x24 => "Home".into(),
        0x23 => "End".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x26 => "Up".into(),
        0x28 => "Down".into(),
        0x25 => "Left".into(),
        0x27 => "Right".into(),
        0x2C => "PrintScreen".into(),
        0xC0 => "`".into(),
        v if (0x41..=0x5A).contains(&v) => {
            char::from_u32(v).unwrap_or('?').to_string()
        }
        v if (0x30..=0x39).contains(&v) => {
            char::from_u32(v).unwrap_or('?').to_string()
        }
        v if (0x70..=0x87).contains(&v) => format!("F{}", v - 0x6F),
        v => format!("VK{v:02X}"),
    }
}
