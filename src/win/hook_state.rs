//! State machine ของ "โหมด Mac" — แยก logic ออกจาก Win32 เพื่อให้เทสต์ได้จริง
//!
//! หน้าที่: แปลงปุ่ม `Alt` ซ้าย (ซึ่งผู้ใช้ตั้งใจให้ทำหน้าที่เป็น ⌘) ให้แอปที่ focus อยู่
//! เห็นเป็น `Ctrl+<ปุ่ม>` โดยไม่ต้องพึ่ง Win32 เลย → เทสต์สถานการณ์ได้ครบ
//!
//! # กติกาความปลอดภัย (สำคัญมาก)
//!
//! ถ้าเรา inject `Ctrl` ลงไปแล้วไม่มีโอกาสปล่อย → `Ctrl` ค้างอยู่ทั้งระบบ
//! → ผู้ใช้พิมพ์ไม่ได้ (ทุกตัวอักษรกลายเป็น `Ctrl+<ตัวอักษร>`) และ
//! `Alt+Space` กลายเป็น `Ctrl+Alt+Space` ซึ่งไม่ตรงกับ hotkey ที่ลงทะเบียนไว้
//! → ใช้ไม่ได้ทั้งคีย์ลัด
//!
//! จุดที่ทำให้เกิดบั๊กนี้ได้จริง (เคยเกิดในเวอร์ชันก่อนหน้า):
//! 1. นับ "ปุ่มที่กดค้าง" ด้วยการนับ event → auto-repeat ส่ง `keydown` ซ้ำแต่
//!    `keyup` มีครั้งเดียว → ตัวเลขไม่กลับเป็น 0 → `Ctrl` ไม่ถูกปล่อย
//! 2. `keyup` ของ `Alt` หายไป (เช่นระหว่าง UAC / secure desktop / hook ถูกถอด)
//!    → state ค้างว่า `Alt` ยังกดอยู่ → ทุกปุ่มที่กดต่อไปกลายเป็น `Ctrl+<ปุ่ม>`
//! 3. โปรแกรมถูกปิด/ตายขณะผู้ใช้กด `Alt` ค้าง → `Ctrl` ที่ inject ไว้ไม่มีโอกาสถูกปล่อย
//!
//! ทั้งสามกรณีถูกจัดการในไฟล์นี้

/// VK ของปุ่ม Alt ซ้าย (low-level hook รายงาน VK_LMENU ไม่ใช่ VK_MENU)
pub const VK_LMENU: u32 = 0xA4;
/// VK ของปุ่ม Alt ขวา
pub const VK_RMENU: u32 = 0xA5;
/// VK ของปุ่ม Ctrl ซ้าย
pub const VK_LCONTROL: u32 = 0xA2;
/// VK ของปุ่ม Ctrl ขวา
pub const VK_RCONTROL: u32 = 0xA3;
/// VK ของปุ่ม Ctrl (รวมซ้าย+ขวา)
pub const VK_CONTROL: u32 = 0x11;
/// VK ของปุ่ม Alt (รวมซ้าย+ขวา)
pub const VK_MENU: u32 = 0x12;
/// VK ของปุ่ม Tab
pub const VK_TAB: u32 = 0x09;
/// VK ของปุ่ม Space
pub const VK_SPACE: u32 = 0x20;
/// VK ของปุ่ม Win ซ้าย
pub const VK_LWIN: u32 = 0x5B;

/// ถ้าไม่มี heartbeat ของ `Alt` นานเกินนี้ (มิลลิวินาที) → เดาว่า `keyup` ของ Alt หาย
///
/// ปกติ Windows จะส่ง auto-repeat ของปุ่ม modifier มาเป็น `keydown` ซ้ำ ๆ
/// ตอนผู้ใช้กดค้าง → ใช้เป็น "หัวใจ" ยืนยันว่าปุ่มยังถูกกดอยู่จริง
pub const ALT_STALE_MS: u64 = 2_500;

/// ถ้าไม่เคยเห็น auto-repeat ของ Alt เลย (คีย์บอร์ด/ไดรเวอร์ไม่ repeat modifier)
/// → ใช้เวลาผ่อนยาวกว่า เพื่อไม่ให้เกิด false positive
pub const ALT_STALE_MS_NO_REPEAT: u64 = 15_000;

/// หน้าต่างที่ถือว่า heartbeat ยัง "สด" (ใช้ตัดสินว่าผู้ใช้ยังกด Alt อยู่จริง)
pub const HEARTBEAT_WINDOW_MS: u64 = 400;

/// เหตุการณ์คีย์ที่ hook สังเกตได้
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    KeyDown(u32),
    KeyUp(u32),
}

/// สิ่งที่ต้องทำกับระบบจริงหลังประมวลผลเหตุการณ์
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// กด `Ctrl` ซ้ายลง (แทน `Alt`)
    CtrlDown,
    /// ปล่อย `Ctrl` ซ้าย
    CtrlUp,
    /// ส่ง "Alt up" ปลอม เพื่อล้างสถานะ `Alt` ที่ระบบบันทึกไว้
    ///
    /// จำเป็นเพราะ Windows อัปเดต async key state **ก่อน** เรียก low-level hook
    /// ถ้าไม่ล้าง ระบบจะยัง "เห็น" ว่า Alt กดอยู่ → `RegisterHotKey` จะไม่ยอมรับ
    /// `Ctrl+Space` เพราะกลายเป็น `Ctrl+Alt+Space`
    AltUp,
    /// กด `Alt` ซ้ายลง **จริง ๆ** (ผ่านไปยังระบบ) เพื่อเปิด task switcher
    ///
    /// ต้อง "ปล่อยผ่าน" (ไม่ใช่กลืน) ไม่งั้น task switcher ของ Windows จะไม่เปิด
    SwitcherAltDown,
    /// กด `Tab` แล้วปล่อยทันที (เลื่อนตัวที่เลือกใน task switcher)
    SwitcherTab,
    /// ปล่อย `Alt` ซ้ายจริง ๆ → task switcher ปิดและยืนยันตัวที่เลือก
    SwitcherAltUp,
    /// ส่ง `Win+Space` ให้ Windows สลับภาษาเอง (ใช้กับ ⌘+Space)
    WinSpace,
}

/// สถานะของโหมด Mac ทั้งหมด (pure logic — ไม่ติดต่อ Win32 เลย)
#[derive(Debug, Clone)]
pub struct HookState {
    /// เปิด/ปิดโหมด Mac
    pub enabled: bool,
    /// ผู้ใช้กด `Alt` ซ้ายอยู่ (ตามที่ hook เห็น)
    alt_down: bool,
    /// เรา inject `Ctrl` ลงไปแล้ว
    ctrl_down: bool,
    /// ปุ่มอื่นที่กดค้างอยู่ตอน `Alt` ค้าง — เก็บ **รายปุ่ม** ไม่ใช่นับ event
    held: [bool; 256],
    /// จำนวนปุ่มที่ยังกดอยู่จริง
    held_count: u32,
    /// ผู้ใช้กด `Ctrl` จริง (ซ้าย/ขวา) อยู่หรือไม่ — ใช้แยก "Ctrl ที่เรา inject"
    /// ออกจาก "Ctrl ที่ผู้ใช้กดเอง"
    pub phys_ctrl_down: bool,
    /// ผู้ใช้กด `Alt` ซ้ายจริงอยู่หรือไม่ (ดูเป็นรายฝั่ง)
    alt_left_down: bool,
    /// ผู้ใช้กด `Alt` ขวาจริงอยู่หรือไม่ (AltGr / ปุ่ม Alt ขวา)
    alt_right_down: bool,
    /// เวลาที่เห็นเหตุการณ์ใด ๆ ล่าสุด
    last_event_ms: u64,
    /// เวลาที่เห็นเหตุการณ์ของ `Alt` ล่าสุด (heartbeat)
    last_alt_ms: u64,
    /// เคยเห็น auto-repeat ของ `Alt` หรือยัง (ถ้าเคยเห็น → ใช้ timeout ตรวจ keyup หายได้)
    alt_repeat_seen: bool,
    /// สงสัยว่า `keyup` ของ `Alt` หายไป (รอ heartbeat ยืนยัน)
    alt_suspect: bool,
    /// เรากด `Alt` ไว้ให้ task switcher เปิดอยู่ (ผู้ใช้ยังกด ⌘ อยู่)
    switcher_open: bool,
    /// กด `Space` ที่ทำงานในโหมด ⌘ (เปลี่ยนภาษา) — ต้องกลืนทั้ง down และ up
    mac_space: bool,
}

impl Default for HookState {
    fn default() -> Self {
        Self::new()
    }
}

impl HookState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            alt_down: false,
            ctrl_down: false,
            held: [false; 256],
            held_count: 0,
            phys_ctrl_down: false,
            alt_left_down: false,
            alt_right_down: false,
            last_event_ms: 0,
            last_alt_ms: 0,
            alt_repeat_seen: false,
            alt_suspect: false,
            switcher_open: false,
            mac_space: false,
        }
    }

    /// ผู้ใช้กด `Alt` อยู่หรือไม่ (ใช้รายงานสถานะ)
    pub fn is_alt_down(&self) -> bool {
        self.alt_down
    }

    /// เรา inject `Ctrl` ค้างอยู่หรือไม่
    pub fn is_ctrl_injected(&self) -> bool {
        self.ctrl_down
    }

    /// จำนวนปุ่มอื่นที่กดค้างอยู่
    pub fn held_count(&self) -> u32 {
        self.held_count
    }

    /// สงสัยว่า `keyup` ของ Alt หายไปหรือเปล่า (รอ heartbeat ยืนยัน)
    pub fn is_alt_suspect(&self) -> bool {
        self.alt_suspect
    }

    /// task switcher ที่เราเปิดไว้ยังค้างอยู่หรือไม่ (เรากด Alt ไว้ให้)
    #[allow(dead_code)]
    pub fn is_switcher_open(&self) -> bool {
        self.switcher_open
    }

    /// `Space` ที่กดในโหมด ⌘ (ใช้เปลี่ยนภาษา) กำลัง "ค้าง" อยู่หรือไม่
    ///
    /// ต้องกลืนทั้ง `keydown` และ `keyup` ของ Space (ไม่งั้นแอปจะได้ keyup โดยไม่มี keydown)
    pub fn is_mac_space(&self) -> bool {
        self.mac_space
    }

    /// ผู้ใช้กด `Alt` จริงอยู่หรือไม่ (ซ้ายหรือขวา)
    ///
    /// ใช้โดย watchdog: ถ้าระบบเห็น `Alt` ค้าง แต่ผู้ใช้ไม่ได้กด `Alt` จริง
    /// → แปลว่าค้างจากการ inject → ปล่อยให้
    ///
    /// ต้องดูทั้งซ้ายและขวา ถ้าดูแค่ซ้าย → ผู้ใช้ที่กด `Alt` ขวา (AltGr)
    /// จะถูกปล่อยทิ้งโดยไม่ตั้งใจ (พิมพ์ภาษาไทย/อังกฤษไม่ได้)
    pub fn is_phys_alt_down(&self) -> bool {
        self.alt_left_down || self.alt_right_down
    }

    /// จดว่าผู้ใช้กด/ปล่อยปุ่ม modifier จริง (เรียกจาก hook ทุกครั้งที่เห็นเหตุการณ์)
    ///
    /// แยกจาก [`HookState::handle`] เพราะ hook ยังต้องรู้ว่าผู้ใช้กด `Alt`/`Ctrl` อยู่จริง
    /// แม้ตอน **โหมด Mac ปิดอยู่** (watchdog ใช้ข้อมูลนี้ตัดสินว่าปุ่มค้างควรปล่อยหรือไม่)
    pub fn note_physical(&mut self, vk: u32, down: bool) {
        match vk {
            VK_LCONTROL | VK_RCONTROL => self.phys_ctrl_down = down,
            VK_LMENU => self.alt_left_down = down,
            VK_RMENU => self.alt_right_down = down,
            _ => {}
        }
    }

    /// ประมวลผลเหตุการณ์คีย์ → คืนรายการสิ่งที่ต้องทำ
    pub fn handle(&mut self, ev: Event, now_ms: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.last_event_ms = now_ms;

        match ev {
            Event::KeyDown(vk) => {

                if vk == VK_LMENU {
                    self.last_alt_ms = now_ms;
                    if self.alt_down {
                        // auto-repeat ของ Alt → คือ heartbeat ที่บอกว่าผู้ใช้ยังกดอยู่
                        self.alt_repeat_seen = true;
                        // ไม่ต้องล้างสถานะ Alt ซ้ำ — ทำแค่ครั้งแรกพอ
                        // (ทุกครั้งที่ inject คือ SendInput ใน hook callback = ช้า)
                        return fx;
                    }
                    // กด Alt ครั้งใหม่ → ล้างสภาพค้างจากรอบก่อนหน้า
                    self.clear_held();
                    self.alt_suspect = false;
                    if self.ctrl_down {
                        self.ctrl_down = false;
                        fx.push(Effect::CtrlUp);
                    }
                    self.alt_down = true;
                    fx.push(Effect::AltUp);
                    return fx;
                }

                if self.alt_down {
                    if self.alt_suspect {
                        // ต้องตัดสินว่า Alt ยังถูกกดอยู่จริงไหม → ดู heartbeat
                        if now_ms.saturating_sub(self.last_alt_ms) <= HEARTBEAT_WINDOW_MS {
                            // heartbeat กลับมาแล้ว → ผู้ใช้ยังกด Alt อยู่จริง
                            self.alt_suspect = false;
                        } else {
                            // heartbeat ไม่กลับมา → keyup ของ Alt หายไปจริง
                            // → ปล่อย Ctrl และปล่อยให้ปุ่มนี้ผ่านเป็นปุ่มธรรมดา
                            self.alt_down = false;
                            self.alt_suspect = false;
                            self.clear_held();
                            self.close_switcher(&mut fx);
                            if self.ctrl_down {
                                self.ctrl_down = false;
                                fx.push(Effect::CtrlUp);
                            }
                            return fx;
                        }
                    }

                    // ── ⌘+Tab = app switcher แบบ macOS ────────────────────
                    // ผู้ใช้กด ⌘ ค้างไว้ แล้วกด Tab ไปเรื่อย ๆ เพื่อเลือกหน้าต่าง
                    // ปล่อย ⌘ ค่อย commit (แบบเดียวกับ macOS)
                    //
                    // ตรงนี้ **ไม่ inject Ctrl** เพราะต้องการให้ Windows เห็นเป็น Alt+Tab
                    // (ถ้า inject Ctrl จะกลายเป็น Ctrl+Alt+Tab → task switcher เพี้ยน/ค้าง)
                    if vk == VK_TAB {
                        // auto-repeat ของ Tab (กดค้าง) → ไม่ต้องยิงซ้ำ
                        if !self.held[VK_TAB as usize] {
                            if !self.switcher_open {
                                self.switcher_open = true;
                                fx.push(Effect::SwitcherAltDown);
                            }
                            fx.push(Effect::SwitcherTab);
                        }
                        self.track(vk, true);
                        return fx;
                    }

                    // ── ⌘+Space = เปลี่ยนภาษา (map เป็น Win+Space) ─────────
                    // ไม่ inject Ctrl เพราะต้องการให้ Windows เห็นเป็น Win+Space
                    // (Windows จะสลับภาษาให้เองทั้งเครื่อง ไม่ต้องอยู่ในช่องพิมพ์)
                    if vk == VK_SPACE {
                        if !self.held[VK_SPACE as usize] {
                            // ปล่อย Ctrl ที่ inject ค้างอยู่ก่อน
                            // (ไม่งั้นจะกลายเป็น Ctrl+Win+Space)
                            if self.ctrl_down {
                                self.ctrl_down = false;
                                fx.push(Effect::CtrlUp);
                            }
                            self.mac_space = true;
                            fx.push(Effect::WinSpace);
                        }
                        self.track(vk, true);
                        return fx;
                    }

                    if !self.ctrl_down {
                        self.ctrl_down = true;
                        fx.push(Effect::CtrlDown);
                    }
                    if !is_modifier(vk) {
                        self.track(vk, true);
                    }
                }
            }
            Event::KeyUp(vk) => {

                if vk == VK_LMENU {
                    self.last_alt_ms = now_ms;
                    self.alt_down = false;
                    self.alt_suspect = false;
                    // ปิด task switcher ที่เราเปิดไว้ (ผู้ใช้ปล่อย ⌘ แล้ว → commit ตัวที่เลือก)
                    self.close_switcher(&mut fx);
                    // อย่า clear held ตรงนี้ — ปุ่มที่กดอยู่ยังต้องถูกปล่อยให้เป็นลำดับ
                    // (ถ้าผู้ใช้ปล่อย Alt ก่อนปุ่มอื่น → Ctrl ต้องค้างไว้จนกว่าปุ่มอื่นจะปล่อย)
                    if self.ctrl_down && self.held_count == 0 {
                        self.ctrl_down = false;
                        fx.push(Effect::CtrlUp);
                    }
                    return fx;
                }

                // ต้อง track ทุกปุ่ม (ไม่ใช่เฉพาะตอน Alt ค้าง) เพราะอาจเป็นการปล่อย
                // ปุ่มที่กดอยู่หลังจากปล่อย Alt ไปแล้ว → ต้องนับถอยให้ถูก
                if !is_modifier(vk) {
                    self.track(vk, false);
                }
                if vk == VK_SPACE {
                    self.mac_space = false;
                }

                // ปล่อย Ctrl เมื่อไม่มีปุ่มอื่นค้างและ Alt ไม่ได้กดอยู่แล้ว
                if self.ctrl_down && self.held_count == 0 && !self.alt_down {
                    self.ctrl_down = false;
                    fx.push(Effect::CtrlUp);
                }
            }
        }

        fx
    }

    /// ตรวจสถานะเป็นระยะ (เรียกจาก timer) → คืนรายการสิ่งที่ต้องทำ
    ///
    /// หน้าที่หลัก: ตรวจว่า `keyup` ของ `Alt` หายไปหรือเปล่า
    /// (เช่นระหว่าง UAC / secure desktop / hook ถูก Windows ถอดออก)
    pub fn tick(&mut self, now_ms: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        if !self.enabled || !self.alt_down || self.alt_suspect {
            return fx;
        }

        // ถ้าเคยเห็น auto-repeat ของ Alt → ใช้ timeout ตรวจได้อย่างมั่นใจ
        // ถ้าไม่เคยเห็น → ใช้เวลาผ่อนยาวกว่า (กัน false positive)
        let limit = if self.alt_repeat_seen {
            ALT_STALE_MS
        } else {
            ALT_STALE_MS_NO_REPEAT
        };

        if now_ms.saturating_sub(self.last_alt_ms) > limit {
            // เดาว่า keyup ของ Alt หายไป → รอ heartbeat ยืนยันตอนผู้ใช้กดปุ่มถัดไป
            self.alt_suspect = true;
            // ปล่อย Ctrl ทิ้งเลย (แม้ยังมีปุ่มค้าง) เพราะคอมโบนี้ตายแล้ว
            // ถ้าไม่ปล่อย → Ctrl ค้างทั้งระบบ → พิมพ์ไม่ได้
            if self.ctrl_down {
                self.ctrl_down = false;
                fx.push(Effect::CtrlUp);
            }
            // ปิด task switcher ด้วย (เรากด Alt ไว้ให้) ไม่งั้น Alt ค้างในระบบ
            self.close_switcher(&mut fx);
        }

        fx
    }

    /// ล้างสถานะทั้งหมด (ตอนปิดโปรแกรม / ปิดโหมด Mac) → คืนรายการสิ่งที่ต้องทำ
    pub fn reset(&mut self) -> Vec<Effect> {
        let mut fx = Vec::new();
        if self.ctrl_down {
            self.ctrl_down = false;
            fx.push(Effect::CtrlUp);
        }
        self.close_switcher(&mut fx);
        self.mac_space = false;
        self.alt_down = false;
        self.alt_suspect = false;
        self.alt_repeat_seen = false;
        self.clear_held();
        fx
    }

    /// ปิด task switcher ที่เราเปิดไว้ (ถ้าเปิดอยู่) → ปล่อย `Alt` ที่เรากดไว้
    ///
    /// ต้องทำทุกครั้งที่ ⌘ "หลุด" (ปล่อย, หายไป, ปิดโหมด Mac, โปรแกรมจบ)
    /// ไม่งั้น `Alt` จะค้างในระบบ → task switcher ค้างเปิดไว้ตลอด
    fn close_switcher(&mut self, fx: &mut Vec<Effect>) {
        if self.switcher_open {
            self.switcher_open = false;
            fx.push(Effect::SwitcherAltUp);
        }
    }

    /// เพิ่ม/ลบปุ่มที่กดค้างอยู่ในชุด `held`
    ///
    /// ต้องเป็น "รายปุ่ม" ไม่ใช่การนับ event: auto-repeat ส่ง `keydown` ซ้ำหลายครั้ง
    /// แต่ `keyup` มีแค่ครั้งเดียว ถ้านับเป็นตัวเลข → ตัวเลขไม่กลับเป็น 0
    /// → `Ctrl` ที่ inject ไว้ไม่ถูกปล่อย → ค้างทั้งระบบ
    fn track(&mut self, vk: u32, down: bool) {
        let Some(idx) = usize::try_from(vk).ok().filter(|i| *i < self.held.len()) else {
            return;
        };
        if down {
            if !self.held[idx] {
                self.held[idx] = true;
                self.held_count += 1;
            }
        } else if self.held[idx] {
            self.held[idx] = false;
            self.held_count = self.held_count.saturating_sub(1);
        }
    }

    fn clear_held(&mut self) {
        self.held = [false; 256];
        self.held_count = 0;
    }
}

/// ปุ่มที่เป็น modifier (ไม่ต้องนับว่า "กดค้าง" เพราะปล่อยตาม Alt ได้เลย)
fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0xA0..=0xA5 | 0x5B | 0x5C | 0x5D)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(vk: u32) -> Event {
        Event::KeyDown(vk)
    }
    fn up(vk: u32) -> Event {
        Event::KeyUp(vk)
    }

    const S: u32 = b'S' as u32;

    /// เครื่องมือช่วย: จำลองการกด ⌘+ปุ่มแล้วปล่อย
    fn press_combo(st: &mut HookState, vk: u32, t: &mut u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        fx.extend(st.handle(down(VK_LMENU), *t));
        fx.extend(st.handle(down(vk), *t));
        *t += 10;
        fx.extend(st.handle(up(vk), *t));
        fx.extend(st.handle(up(VK_LMENU), *t));
        *t += 10;
        fx
    }

    #[test]
    fn combo_injects_ctrl_then_releases_it() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        let fx = press_combo(&mut st, S, &mut t);

        assert!(fx.contains(&Effect::CtrlDown), "ต้องกด Ctrl ลง");
        assert!(fx.contains(&Effect::AltUp), "ต้องล้างสถานะ Alt ในระบบ");
        assert!(fx.contains(&Effect::CtrlUp), "ต้องปล่อย Ctrl");
        assert!(!st.is_ctrl_injected(), "หลังจบต้องไม่เหลือ Ctrl ที่ inject ไว้");
        assert!(!st.is_alt_down(), "หลัวจบต้องไม่เหลือ Alt ที่กดอยู่");
    }

    /// auto-repeat ของ Alt ต้อง "ล้างสถานะ Alt" แค่ครั้งแรก
    ///
    /// ทุกครั้งที่ emit `Effect::AltUp` คือการเรียก `SendInput` ใน hook callback
    /// ซึ่งช้าหลายร้อยไมโครวินาที → กด ⌘ ค้างแล้วระบบจะสะดุดตลอดเวลา
    #[test]
    fn alt_repeat_does_not_release_alt_again() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        let first = st.handle(down(VK_LMENU), t);
        assert!(
            first.contains(&Effect::AltUp),
            "กด Alt ครั้งแรกต้องล้างสถานะ Alt"
        );

        // auto-repeat ของ Alt (กดค้าง) → ต้องไม่ emit AltUp อีก
        for _ in 0..30 {
            t += 30;
            let fx = st.handle(down(VK_LMENU), t);
            assert!(
                !fx.contains(&Effect::AltUp),
                "auto-repeat ของ Alt ต้องไม่ inject ซ้ำ (แต่ละครั้งคือ SendInput ที่ช้า)"
            );
        }
    }

    /// บั๊กเดิม: นับ event → auto-repeat ทำให้ตัวเลขไม่กลับเป็น 0 → Ctrl ค้าง
    #[test]
    fn auto_repeat_does_not_leak_ctrl() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        // กด S ค้าง 2 วินาที → keydown ซ้ำ ~30 ครั้ง แต่ keyup ครั้งเดียว
        for _ in 0..30 {
            t += 30;
            st.handle(down(S), t);
        }
        t += 30;
        st.handle(up(S), t);
        st.handle(up(VK_LMENU), t);

        assert_eq!(st.held_count(), 0, "หลังปล่อยต้องไม่มีปุ่มค้าง");
        assert!(!st.is_ctrl_injected(), "Ctrl ต้องถูกปล่อย (ไม่ค้างทั้งระบบ)");
    }

    /// ปล่อย Alt ก่อนปุ่มอื่น → Ctrl ต้องค้างไว้จนกว่าปุ่มอื่นจะปล่อย
    #[test]
    fn alt_released_before_key_keeps_ctrl_until_key_is_released() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(S), t);
        t += 10;
        st.handle(up(VK_LMENU), t);

        assert!(st.is_ctrl_injected(), "Ctrl ต้องยังค้างอยู่ระหว่างที่ปุ่มอื่นยังกด");

        t += 10;
        st.handle(up(S), t);
        assert!(!st.is_ctrl_injected(), "ปล่อยปุ่มสุดท้ายแล้วต้องปล่อย Ctrl");
    }

    /// บั๊กเดิมอีกจุด: auto-repeat ของ Alt เคยไปล้างรายการปุ่มที่กดค้าง
    /// → ทำให้นับผิด → Ctrl ค้าง/หลุดผิดจังหวะ
    #[test]
    fn alt_repeat_does_not_clear_held_keys() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(S), t);
        assert_eq!(st.held_count(), 1);

        // auto-repeat ของ Alt
        t += 600;
        st.handle(down(VK_LMENU), t);
        assert_eq!(st.held_count(), 1, "auto-repeat ของ Alt ต้องไม่ล้างปุ่มที่กดค้าง");

        t += 10;
        st.handle(up(S), t);
        assert_eq!(st.held_count(), 0);
    }

    /// บั๊กที่ทำให้ "พิมพ์ไม่ได้" ตอนนี้: keyup ของ Alt หายไป
    /// → ทุกปุ่มที่กดต่อไปต้องกลายเป็น Ctrl+<ปุ่ม> → พิมพ์ไม่ได้
    #[test]
    fn lost_alt_keyup_is_detected_and_recovered() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        // ผู้ใช้กด ⌘ ค้างแล้วกดปุ่มอื่น (เรา inject Ctrl) — มี auto-repeat = heartbeat
        st.handle(down(VK_LMENU), t);
        st.handle(down(S), t);
        assert!(st.is_ctrl_injected(), "ต้อง inject Ctrl ไว้");
        for _ in 0..5 {
            t += 100;
            st.handle(down(VK_LMENU), t); // heartbeat
        }
        // ... keyup หาย → ไม่มีเหตุการณ์ต่อมาเลย
        t += ALT_STALE_MS + 500;

        let fx = st.tick(t);
        assert!(fx.contains(&Effect::CtrlUp), "ต้องปล่อย Ctrl ที่ค้างไว้");

        // ผู้ใช้พิมพ์ต่อ → ต้องเป็นปุ่มธรรมดา (ไม่ใช่ Ctrl+<ตัวอักษร>)
        t += 100;
        let fx = st.handle(down(b'A' as u32), t);
        assert!(
            !fx.contains(&Effect::CtrlDown),
            "หลังกู้สถานะแล้ว ปุ่มต้องผ่านเป็นปุ่มธรรมดา ไม่ใช่ Ctrl+A"
        );
        assert!(!st.is_alt_down(), "สถานะ Alt ต้องถูกล้าง");
    }

    /// ถ้า driver ไม่ repeat Alt → heartbeat ไม่มี → ต้องไม่ตัดสินใจผิด (false positive)
    #[test]
    fn no_false_positive_when_alt_never_repeats() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        // ไม่มี heartbeat เลย แต่ยังไม่ถึงเวลาที่ยอมให้ตัดสินใจ
        t += ALT_STALE_MS + 500;
        let fx = st.tick(t);
        assert!(fx.is_empty(), "ยังไม่ควรตัดสินว่า Alt ค้าง (ไม่เคยเห็น auto-repeat)");

        // ผู้ใช้กดปุ่มต่อ → ยังต้องทำงานเป็น ⌘+ปุ่ม
        t += 10;
        let fx = st.handle(down(S), t);
        assert!(fx.contains(&Effect::CtrlDown), "ต้องยัง inject Ctrl ได้");
    }

    /// ตรวจจับ keyup หายได้เมื่อ driver repeat Alt (มี heartbeat มาแล้ว)
    #[test]
    fn stale_alt_detected_after_heartbeat_seen() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(S), t); // เรา inject Ctrl ไว้
        t += 100;
        st.handle(down(VK_LMENU), t); // heartbeat → alt_repeat_seen = true

        t += ALT_STALE_MS + 100;
        let fx = st.tick(t);
        assert!(
            fx.contains(&Effect::CtrlUp),
            "ต้องปล่อย Ctrl ที่ค้างไว้ทันที"
        );
    }

    /// keyup ของปุ่มที่ไม่เคยถูกกด → ต้องไม่ทำให้ตัวเลขติดลบ
    #[test]
    fn unknown_keyup_is_safe() {
        let mut st = HookState::new();
        st.enabled = true;
        let t = 0u64;

        st.handle(up(b'X' as u32), t);
        assert_eq!(st.held_count(), 0);
    }

    /// ปิดโปรแกรม/ปิดโหมด Mac ต้องปล่อย Ctrl ที่ inject ไว้เสมอ
    #[test]
    fn reset_releases_injected_ctrl() {
        let mut st = HookState::new();
        st.enabled = true;
        let t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(S), t);

        let fx = st.reset();
        assert!(fx.contains(&Effect::CtrlUp), "ต้องปล่อย Ctrl");
        assert!(!st.is_ctrl_injected());
    }

    /// watchdog ต้องรู้ว่าผู้ใช้กด Alt จริงอยู่หรือไม่ — ทั้งซ้ายและขวา
    ///
    /// ถ้าดูแค่ฝั่งซ้าย → ผู้ใช้ที่กด Alt ขวา (AltGr) จะถูก watchdog ปล่อยทิ้ง
    /// → พิมพ์ตัวอักษรที่ต้องใช้ AltGr ไม่ได้ (เช่น @ { } ในคีย์บอร์ดยุโรป)
    #[test]
    fn physical_alt_tracks_both_sides() {
        let mut st = HookState::new();

        // ยังไม่กดอะไร
        assert!(!st.is_phys_alt_down());

        // กด Alt ขวาอย่างเดียว (AltGr) → ต้องถือว่า "ผู้ใช้กดอยู่"
        st.note_physical(VK_RMENU, true);
        assert!(
            st.is_phys_alt_down(),
            "Alt ขวาต้องนับเป็นปุ่มที่ผู้ใช้กดจริง"
        );
        // ... และโหมด Mac ยังไม่ควรถือว่า "⌘ ถูกกด" (Alt ขวาไม่ใช่ ⌘)
        assert!(!st.is_alt_down(), "Alt ขวาไม่ควรทำให้โหมด Mac ทำงาน");

        // ปล่อย Alt ขวา
        st.note_physical(VK_RMENU, false);
        assert!(!st.is_phys_alt_down());

        // Alt ซ้าย
        st.note_physical(VK_LMENU, true);
        assert!(st.is_phys_alt_down());
        st.note_physical(VK_LMENU, false);
        assert!(!st.is_phys_alt_down());
    }

    /// Ctrl จริงต้องถูกแยกออกจาก Ctrl ที่เรา inject
    #[test]
    fn physical_ctrl_is_tracked() {
        let mut st = HookState::new();

        assert!(!st.phys_ctrl_down);
        st.note_physical(VK_LCONTROL, true);
        assert!(st.phys_ctrl_down, "Ctrl ซ้ายที่ผู้ใช้กดต้องถูกจด");
        st.note_physical(VK_LCONTROL, false);
        assert!(!st.phys_ctrl_down);
    }
    /// ⌘+Tab ต้องทำเป็น app switcher แบบ macOS: กด ⌘ ค้าง แล้ว Tab ไปเรื่อย ๆ ได้
    ///
    /// อาการเดิม: กด ⌘+Tab แล้วระบบปล่อย Alt ทันที → task switcher ปิดก่อนเวลา
    /// → ผู้ใช้กด Tab ต่อเพื่อเลือกหน้าต่างไม่ได้
    #[test]
    fn command_tab_keeps_switcher_open_until_command_released() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        // กด ⌘ (Alt ซ้าย)
        let fx = st.handle(down(VK_LMENU), t);
        assert!(fx.contains(&Effect::AltUp));

        // กด Tab ครั้งแรก → เปิด task switcher (กด Alt จริง + ยิง Tab)
        let fx = st.handle(down(VK_TAB), t);
        assert!(
            fx.contains(&Effect::SwitcherAltDown),
            "ต้องกด Alt จริงเพื่อเปิด task switcher"
        );
        assert!(fx.contains(&Effect::SwitcherTab), "ต้องยิง Tab ไปยัง task switcher");
        assert!(
            !fx.contains(&Effect::CtrlDown),
            "⌘+Tab ต้องไม่ inject Ctrl (จะกลายเป็น Ctrl+Alt+Tab → task switcher เพี้ยน)"
        );
        assert!(st.is_switcher_open(), "task switcher ต้องค้างเปิดอยู่");

        // ปล่อย Tab แล้วกดใหม่อีกครั้ง → เลื่อนตัวที่เลือก (ไม่ต้องกด Alt ซ้ำ)
        t += 50;
        st.handle(up(VK_TAB), t);
        t += 50;
        let fx = st.handle(down(VK_TAB), t);
        assert!(fx.contains(&Effect::SwitcherTab), "กด Tab ครั้งที่สองต้องเลื่อนตัวที่เลือก");
        assert!(
            !fx.contains(&Effect::SwitcherAltDown),
            "ต้องไม่กด Alt ซ้ำ (task switcher เปิดอยู่แล้ว)"
        );

        // ปล่อย ⌘ → ปิด task switcher (commit ตัวที่เลือก)
        t += 50;
        st.handle(up(VK_TAB), t);
        t += 50;
        let fx = st.handle(up(VK_LMENU), t);
        assert!(
            fx.contains(&Effect::SwitcherAltUp),
            "ปล่อย ⌘ ต้องปล่อย Alt ที่เรากดไว้ (ไม่งั้น task switcher ค้าง)"
        );
        assert!(!st.is_switcher_open());
    }

    /// ถ้า ⌘ หายไป (keyup ไม่มา) ต้องปิด task switcher และปล่อย Alt ที่เรากดไว้
    #[test]
    fn command_tab_switcher_is_closed_on_reset() {
        let mut st = HookState::new();
        st.enabled = true;
        let t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(VK_TAB), t);
        assert!(st.is_switcher_open());

        // ปิดโปรแกรม / ปิดโหมด Mac → ต้องคืน Alt ที่เรากดไว้เสมอ
        let fx = st.reset();
        assert!(
            fx.contains(&Effect::SwitcherAltUp),
            "ต้องปล่อย Alt ที่กดไว้ให้ task switcher (ไม่งั้น Alt ค้างในระบบ)"
        );
        assert!(!st.is_switcher_open());
    }

    /// Tab ที่กดค้าง (auto-repeat) ต้องไม่ยิงซ้ำ (ไม่งั้นเลื่อนข้ามหลายหน้าต่าง)
    #[test]
    fn command_tab_auto_repeat_does_not_fire_again() {
        let mut st = HookState::new();
        st.enabled = true;
        let mut t = 0u64;

        st.handle(down(VK_LMENU), t);
        st.handle(down(VK_TAB), t);

        for _ in 0..10 {
            t += 30;
            let fx = st.handle(down(VK_TAB), t);
            assert!(
                !fx.contains(&Effect::SwitcherTab),
                "auto-repeat ของ Tab ต้องไม่ยิงซ้ำ"
            );
        }
    }

    /// ปุ่ม Alt ต้องถูก "กลืน" เสมอ (hook จะกลืนทั้ง down และ up)
    /// — ทดสอบที่ระดับ state machine ว่า effect ถูกส่งครบทุกครั้ง
    #[test]
    fn alt_always_clears_system_state() {
        let mut st = HookState::new();
        st.enabled = true;
        let t = 0u64;

        let fx = st.handle(down(VK_LMENU), t);
        assert!(fx.contains(&Effect::AltUp));

        let fx = st.handle(up(VK_LMENU), t);
        // keyup ของ Alt ไม่ต้องส่ง AltUp ซ้ำ (สถานะ Alt ถูกล้างไปแล้ว)
        assert!(!fx.contains(&Effect::AltUp));
    }

    /// ⌘+Space = เปลี่ยนภาษา โดย map เป็น `Win+Space` (ให้ Windows สลับเองทั้งเครื่อง)
    ///
    /// เดิมใช้ `Ctrl+Space` → ส่ง `WM_INPUTLANGCHANGEREQUEST` ให้หน้าต่างที่โฟกัส
    /// ซึ่งทำงานเฉพาะตอนอยู่ในช่องพิมพ์ (แอปอื่นไม่รับข้อความนั้น)
    #[test]
    fn command_space_maps_to_win_space() {
        let mut st = HookState::new();
        st.enabled = true;
        let t = 0u64;

        let fx = st.handle(down(VK_LMENU), t);
        let fx2 = st.handle(down(VK_SPACE), t);
        let all: Vec<Effect> = fx.into_iter().chain(fx2).collect();

        assert!(
            all.contains(&Effect::WinSpace),
            "⌘+Space ต้องส่ง Win+Space ให้ Windows สลับภาษาเอง"
        );
        assert!(
            !all.contains(&Effect::CtrlDown),
            "⌘+Space ต้องไม่ inject Ctrl (จะกลายเป็น Ctrl+Win+Space)"
        );
        assert!(
            st.is_mac_space(),
            "ต้องจำไว้ว่า Space นี้ถูกจัดการในโหมด ⌘ (เพื่อกลืน keyup ด้วย)"
        );
        assert!(!st.is_ctrl_injected(), "ไม่ต้องเหลือ Ctrl ที่ inject ไว้");

        // ปล่อย Space → ธงต้องถูกล้าง
        st.handle(up(VK_SPACE), t);
        assert!(!st.is_mac_space());
    }
}
