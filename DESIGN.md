# MacKeys — แผนออกแบบ Rust app สำหรับคีย์ลัดสไตล์ macOS บน Windows

สถานะ: **v1 พัฒนาเสร็จและทดสอบจริงบนเครื่องแล้ว** (ดูรายละเอียดโค้ดจริงใน `README.md`)

> **สิ่งที่ต่างจากแผนเดิม (ตามที่ผู้ใช้เลือก):**
> 1. ใช้ `RegisterHotKey` แทน low-level hook → ไม่มีความเสี่ยง input lag (ตัดสินใจข้อ 2)
> 2. `Ctrl+Tab` ส่งต่อ `Alt+Tab` ของ Windows ไม่ได้ทำ overlay switcher เอง (ตัดสินใจข้อ 2)
> 3. มี settings GUI (egui) ตั้งแต่แรก (ตัดสินใจข้อ 3)
> 4. `Alt+Space` override ทั้งระบบ (ตัดสินใจข้อ 4)
>
> **ผลการทดสอบบนเครื่องจริง:** `Ctrl+Space` สลับภาษาได้จริง (en-US ↔ th-TH) · จับภาพทั้งจอ + clipboard ได้ · `Alt+Space` **ถูก PowerToys Run จองไว้** (ต้องเปลี่ยนคีย์ลัดของ PowerToys ก่อน)

> **สิ่งที่ต่างจากร่างแผนในเอกสารนี้ (ปรับตามโค้ดจริง):**
> 1. ไม่ใช้ `tray-icon` / `muda` — เขียน tray เองด้วย `Shell_NotifyIconW` + `CreatePopupMenu` + `TrackPopupMenu` เพื่อคุม dependency
> 2. ไม่ใช้ `screenshots` crate — ใช้ GDI `BitBlt` เอง (ต้องจับทั้ง virtual desktop 2 จอ)
> 3. ไม่ใช้ `build.rs` + `winres` — สร้างไอคอนเป็น DIB ในตอนรัน ไม่ต้อง embed ไฟล์ `.ico`
> 4. `cli.rs` ใช้ subcommand (`mackeys diagnose`) แทน flag (`mackeys --diagnose`)
> 5. เพิ่ม `gui.rs` + `runner.rs` เพื่อแยก thread (main = eframe, win = message loop, worker = action)
>
> **ผลการทดสอบบนเครื่องจริง (Windows 11 26H2, เสร็จสมบูรณ์):**
> `Ctrl+Space` สลับภาษาได้จริง · `Ctrl+Shift+3` จับภาพทั้งจอ + clipboard ได้ ·
> `Ctrl+Shift+4` ส่งต่อ `Win+Shift+S` แล้ว Snipping Tool เปิดจริง · `Ctrl+Shift+5` เปิดหน้าต่าง Snipping Tool ได้ ·
> `Ctrl+Tab` สลับหน้าต่างได้จริง · เมนู tray คลิกขวาแล้วครบ 4 รายการและทุกรายการทำงาน ·
> หน้าต่างตั้งค่าเปิดได้และภาษาไทยแสดงผลครบ · `Alt+Space` ยังชนกับ PowerToys Run


---

## 1. เป้าหมาย

ย้ายความสามารถจากสคริปต์ AutoHotkey (`langswitch.ahk`) ไปเป็นโปรแกรม Rust ที่:

1. ทำให้คีย์ลัดบน Windows ทำงานเหมือน macOS
2. **สลับภาษาอินพุตด้วย `Alt+Space`** แทน `Win+Space` (ข้อกำหนดหลักของผู้ใช้)
3. ตั้งค่าได้, เปิด/ปิดได้, รันพร้อมกับ Windows ได้

### 1.1 ตารางคีย์ลัด (เดิมจาก AHK → ใหม่)

| macOS | AHK เดิม | เวอร์ชัน Rust | หมายเหตุ |
|---|---|---|---|
| `Ctrl+Space` (สลับ input source) | `Ctrl+Space` → `Win+Space` | `Alt+Space` (+ `⌘+Space` ในโหมด Mac) | ส่ง `Win+Space` ให้ Windows สลับเอง → ทำงานทุกที่ ไม่ต้องอยู่ในช่องพิมพ์ |
| `Cmd+Tab` (สลับแอป) | `Ctrl+Tab` → `Alt+Tab` | `Ctrl+Tab` + `⌘+Tab` ในโหมด Mac | โหมด Mac: hook จัดการเอง (กด `⌘` ค้าง + `Tab` ไปเรื่อย ๆ แบบ macOS) |
| `Cmd+Shift+3` (จับทั้งจอ) | `Ctrl+Shift+3` → `Win+Shift+S` | `Ctrl+Shift+3` | แบบ macOS จริง: บันทึกไฟล์ + คัดลอก clipboard |
| `Cmd+Shift+4` (จับบริเวณ) | `Ctrl+Shift+4` → `Win+Shift+S` | `Ctrl+Shift+4` | ส่งต่อ `Win+Shift+S` ของ Windows (เสถียรกว่าเขียนเอง) |
| `Cmd+Shift+5` (แผงจับภาพ) | `Ctrl+Shift+5` → `ms-screenclip:` | `Ctrl+Shift+5` | เปิด Snipping Tool ผ่าน `ms-screensketch:` |

---

## 2. ปัญหาหลักของ `Alt+Space` ที่ต้องออกแบบรอบ

`Alt+Space` **ไม่ว่างเปล่า** โดยค่าเริ่มต้นมันคือ System Menu ของหน้าต่างที่ active (คลิกขวามุมบนซ้าย → กด `Alt+Space`)

สิ่งที่ต้องตัดสินใจ:

| ประเด็น | ทางเลือก | ผลกระทบ |
|---|---|---|
| ดัก `Alt+Space` ทั้งระบบ | `RegisterHotKey(MOD_ALT, VK_SPACE)` | ปุ่ม System Menu ใช้ไม่ได้ทั่วทั้งเครื่อง — ต้องมีทางปิดใน config |
| ปล่อยให้ System Menu ยังใช้ได้ | ผูก `Ctrl+Alt+Space` แทน | ไม่ตรงคำขอ แต่ปลอดภัยกว่า |
| ปล่อยให้เฉพาะบางโปรแกรม | ตรวจ `GetForegroundWindow()` | ซับซ้อนขึ้น แต่ยืดหยุ่นที่สุด |

**ข้อเสนอ:** ใช้ `Alt+Space` เป็นค่าเริ่มต้น (ตรงตามคำขอ) แต่ทำเป็น *binding ที่ปิดได้* ในไฟล์ config และมีสวิตช์ใน tray เมนู เพื่อผู้ใช้ถอยกลับได้ทันที

> ข้อมูลอ้างอิง: `RegisterHotKey` ดักคอมโบ Alt+Space ได้จริงและ "บัง" System Menu ไปในช่วงที่แอปยังรันอยู่ (พฤติกรรมเดียวกับที่ PowerToys Run ใช้) — `RegisterHotKey` จะไม่ส่งคอมโบนั้นต่อให้หน้าต่างเจ้าของฟอเกัส

### 2.1 ความเสี่ยงอีกตัว: PowerToys

PowerToys "Run" ใช้ `Alt+Space` เป็นค่าเริ่มต้น → ถ้าติดตั้งไว้ `RegisterHotKey` จะ **ล้มเหลว** (first-come-first-served)
แอปต้องตรวจผลของ `RegisterHotKey` และแสดงคำเตือนใน tray พร้อมบอก combo ที่ใช้ได้แทน

---

## 3. สถาปัตยกรรม

แนวทาง: **แยก 2 ชั้น** — ชั้น "Hotkey Engine" (จับคีย์) กับชั้น "Action" (สิ่งที่ต้องทำ) ทำให้เพิ่มคีย์ลัดใหม่ได้โดยไม่แตะโค้ด Win32

```
┌──────────────────────────────────────────────────────┐
│  main.rs        CLI + single-instance + startup      │
├──────────────────────────────────────────────────────┤
│  tray.rs        ไอคอนซิสเตมทรย + เมนู (Win32 ล้วน)     │
├──────────────────────────────────────────────────────┤
│  engine.rs      ตัวแปลง key event → ตัดสินใจ → action │
│                 • ตาราง binding จาก config           │
│                 • ตัดสินใจ "กิน key หรือปล่อยผ่าน"       │
├───────────────┬──────────────────┬───────────────────┤
│ hotkey.rs +   │ win/layout.rs    │ win/capture.rs    │
│ win/hotkey_   │ สลับภาษาอินพุต    │ จับภาพหน้าจอ      │
│ register.rs   │ Win32 input lang  │ GDI BitBlt +      │
│ RegisterHotKey │                  │ arboard (raw RGBA)│
├───────────────┴──────────────────┴───────────────────┤
│  win/  ── layer ที่ห่อ Win32 (windows crate)         │
│  inject.rs (SendInput)   icon.rs (DIB → HICON)      │
│  util.rs   (cursor, shell_open, open_folder)         │
└──────────────────────────────────────────────────────┘
```

### 3.0 โครงสร้าง thread (ของจริง)

โปรแกรมรันใน **process เดียว** แต่แยก 3 thread:

| Thread | หน้าที่ | ไฟล์ |
|---|---|---|
| main | eframe — หน้าต่างตั้งค่า (ซ่อนไว้ตอนเริ่มถ้ามีแต่ tray) | `gui.rs` |
| win | message loop + system tray + `RegisterHotKey` + **hook** (`WH_KEYBOARD_LL`) | `runner.rs`, `tray.rs`, `win/hook.rs` |
| worker | ทำ action (สลับภาษา / จับภาพ) ครอบด้วย `catch_unwind` | `engine.rs` |
| mackeys-log | เขียนไฟล์ log ทั้งหมด (ไม่แตะดิสก์จาก thread อื่น) | `logging.rs` |

> **กฎเหล็กของ thread ที่ติดตั้ง hook:** ห้ามทำงานที่บล็อก (I/O, รอ) บน thread นี้
> เพราะ hook callback ทำงานบน thread เดียวกับ message loop
> ถ้า thread ค้าง → hook "ตายเงียบ" → ผู้ใช้กดอะไรก็ไม่มีผล (แต่โปรแกรมยังทำงานอยู่)
> การเขียน log จึงย้ายไป thread แยก (`mackeys-log`) ที่รับผ่านคิวแบบไม่ block
>
> **กู้จาก hook ตาย:** ถ้า hotkey ยิง (ระบบเห็นอินพุต) แต่ hook ไม่เห็นเหตุการณ์เกิน 2 วินาที
> → ถือว่า hook ถูก Windows ถอด → ติดตั้งใหม่ให้อัตโนมัติ (`hook_is_dead` + `install()`)

> **ข้อควรระวัง (เจอจากการทดสอบจริง):** `TrackPopupMenu` เป็น modal loop ที่ pump ข้อความกลับเข้า
> `wnd_proc` ของเรา ถ้าตอนเรียกยังถือ `RefCell::borrow_mut()` ของ state อยู่ จะเจอ
> `RefCell already borrowed` → panic → เพราะ `wnd_proc` เป็น `extern "system"` ที่ห้าม unwind
> Rust จึง abort ทั้งโปรแกรม ทางแก้คือ **ปล่อย borrow ก่อน** เรียกงานที่ block
> และห่อ `wnd_proc` ทั้งหมดด้วย `catch_unwind` ไว้กันพัง

### 3.1 เลือกกลไกจับคีย์: 2 ระดับ

| | ระดับ A: `RegisterHotKey` | ระดับ B: `WH_KEYBOARD_LL` hook |
|---|---|---|
| ความซับซ้อน | ต่ำ (message-only window + message loop) | สูง (hook callback, จัดการ left/right, repeat, injected flag) |
| คีย์ลัดที่รองรับ | modifier + 1 key เท่านั้น | ทุกแบบ รวมถึง "กด Ctrl ค้าง + กด Tab ซ้ำ" |
| กินคีย์ให้แอปอื่น | ได้ (ไม่ส่งต่อให้ foreground app) | ได้ (return 1 จาก hook) |
| ความเสี่ยง input lag | ไม่มี | มี (hook ค้างเกินเวลา → คีย์บอร์ดทั้งระบบค้าง) |
| ต้องยกระดับสิทธิ์ | ไม่ | ไม่ (แต่ inject เข้า window ที่ยกระดับไม่ได้ — UIPI) |

**ข้อเสนอ: เริ่มด้วยระดับ A** ครอบคลุมงาน 90% (สลับภาษา + จับภาพ 3 คีย์) แล้วค่อยเพิ่มระดับ B เฉพาะตอนที่ทำ app switcher แบบ macOS จริง (ต้อง "กด Ctrl ค้างแล้วแตะ Tab เรื่อย ๆ")

---

## 4. โครงสร้างโค้ด

```
langswitch/
├─ Cargo.toml
├─ README.md
├─ DESIGN.md
└─ src/
   ├─ main.rs                # CLI, single instance, wire ทุกอย่างเข้าด้วยกัน
   ├─ cli.rs                 # clap subcommand: run/diagnose/list-layouts/... + --config
   ├─ config.rs              # โหลด/บันทึก TOML, default, validation
   ├─ hotkey.rs              # ชนิด Hotkey { mods, key } + parser "alt+space" + key name table
   ├─ action.rs              # enum Action + ผูกกับ config
   ├─ engine.rs              # state machine ของ key event → action + worker thread
   ├─ runner.rs              # win thread: window + message loop + ลงทะเบียน hotkey
   ├─ tray.rs                # เมนู system tray (Shell_NotifyIconW + TrackPopupMenu)
   ├─ ui.rs                  # design system (โทนสี/การ์ด/สวิตช์/คีย์แคป/ไอคอนเวกเตอร์)
   ├─ gui.rs                 # หน้าต่างตั้งค่า (egui/eframe)
   ├─ autostart.rs           # HKCU\...\Run
   ├─ singleton.rs           # named mutex
   ├─ logging.rs             # tracing → %LOCALAPPDATA%\MacKeys\logs
   └─ win/
      ├─ mod.rs
      ├─ hotkey_register.rs  # RegisterHotKey / UnregisterHotKey
      ├─ layout.rs           # สลับภาษา (WM_INPUTLANGCHANGEREQUEST)
      ├─ inject.rs           # SendInput (Win+Shift+S, Alt+Tab)
      ├─ capture.rs          # จับภาพเต็มจอ + clipboard
      ├─ hook.rs             # โหมด Mac: WH_KEYBOARD_LL + watchdog
      ├─ hook_state.rs       # state machine ของ hook (ล้วน ๆ ไม่ติดต่อ Win32 → เทสต์ได้)
      ├─ icon.rs             # สร้าง HICON จาก DIB (ตอนรัน ไม่ต้องมีไฟล์ .ico)
      └─ util.rs             # GetCursorPos, shell_open, open_folder
```

### 4.0 ดีไซน์หน้าต่างตั้งค่า

หลักการ: **แยก "ระบบดีไซน์" ออกจาก "เนื้อหา"** เพื่อให้แก้หน้าตามได้เร็วและสม่ำเสมอ

| ไฟล์ | หน้าที่ |
|---|---|
| `src/ui.rs` | `Palette` (โทนสีทั้งหมด) + `apply_style()` + ชิ้นส่วนที่ใช้ซ้ำ: การ์ด, สวิตช์, คีย์แคป, pill, ปุ่ม, โลโก้, ไอคอนเวกเตอร์ |
| `src/gui.rs` | เนื้อหา 4 หน้า (ภาพรวม / คีย์ลัด / ภาษาอินพุต / ระบบ) + เลย์เอาต์ (แถบบน / แถบซ้าย / พื้นที่หลัก) |

**หลักการที่ยึด:**

1. **สีเป็นตัวแปร ไม่กระจัดกระจาย** — ทุกสีมาจาก `Palette` ปรับโทนทั้งแอปแก้ที่เดียว
2. **ไอคอนต้องวาดเอง ไม่ใช้ตัวอักษร** — ฟอนต์ Windows ไม่ครอบคลุมสัญลักษณ์ครบ
   (ตัวที่ไม่มีจะออกมาเป็นกล่อง "tofu") จึงวาดด้วย `egui::Painter` แทน
3. **ฟอนต์ต้องมีสายอักษรสำรอง** — `Tahoma` (ไทย) → `Segoe UI Symbol` (⌘) → `Segoe UI Emoji`
   เรียงใน `FontFamily::Proportional` เดียวกัน ให้ egui fallback เอง
   (มีเทสต์คุมไว้: `gui::tests::font_has_command_symbol`)
4. **เลย์เอาต์กับเนื้อหาแยกกัน** — ใช้ `egui::Panel::top/left()` + `CentralPanel`
   (egui 0.37 ไม่มี `TopBottomPanel`/`SidePanel`
5. **วาดด้วย wgpu (DirectX 12) ไม่ใช่ OpenGL** — เครื่องที่ไม่มี OpenGL ICD ของผู้ผลิตการ์ด
   จะได้แค่ GL 1.1 ของ Microsoft → egui ต้องการ GL 2.0+ → **หน้าต่างขาวโพลน** ทั้งที่โปรแกรมยังทำงานปกติ
   (ตรวจได้ด้วย `Get-Process <แอป> | % Modules` — ถ้าเจอแต่ `OPENGL32.dll` แปลว่าไม่มี ICD)
6. **คอมโบที่ inject เองต้องมี "modifier สะอาด"** — โหมด Mac inject `Ctrl` ค้างไว้ตอนผู้ใช้กด `⌘`
   ถ้ายังกดอยู่ตอนส่งคอมโบที่ไม่มี `Ctrl` ในตัวเอง → ระบบเห็นเป็น `Ctrl+<คอมโบ>` → **คอมโบนั้นไม่ทำงาน**
   (เคยเป็นบั๊กซ้ำ ๆ: `⌘+Q` ไม่ปิดแอป, `⌘+Shift+4` ไม่เปิด Snipping Tool)
   → ทุกคอมโบที่ส่งเองต้องผ่าน `inject::send_clean_combo()` (ปล่อย Ctrl → ส่ง → กดคืน)
   มีเทสต์คุมไว้: `win::inject::tests::combos_without_ctrl_need_clean_modifiers`
7. **คอลัมน์กว้างคงที่ต้องใช้ `allocate_exact_size`** — `allocate_ui_with_layout` จัดพื้นที่เท่าที่ *ใช้จริง*
   → คอลัมน์จะหดตามความยาวข้อความ → แต่ละแถวข้ามกัน ไม่ตรงกัน
   → ใช้ตัวช่วย `ui::fixed_col()` แทน แล้ว)
8. **เลย์เอาต์ต้องมี "สเปก" เป็นตัวเลขกลาง (`ui::layout`)** — ไม่ใช่ตัวเลขมั่ว ๆ ในแต่ละหน้า
   เพราะฉะนั้น "หัวข้อหน้า" กับ "ขอบซ้ายของการ์ด" เคยไม่ตรงกัน (หัวข้อชิดแถบเมนู ส่วนการ์ดเว้นอีก 12 px)
   และรายการเมนูที่เลือกเคยชิดเส้นแบ่งขวาพอดี (ขอบซ้าย 8 px / ขวา 8 px แต่ไฮไลต์โดนตัด)
   → หลักการ: **ขอบซ้ายของทุกอย่างในหน้าเนื้อหาต้องตรงกันหมด**
   → มีเทสต์คุม: `ui::tests::heading_and_cards_share_the_same_left_edge`,
     `ui::tests::nav_item_keeps_padding_on_both_sides`, `ui::tests::layout_tokens_follow_the_spacing_scale`
9. **ช่องกรอกต้องมี "จังหวะ" ของตัวเอง** (อ้างอิงดีไซน์: รีล "Text Field Part 1" ของ `@ui_motion_prompts`)
   - **ป้ายลอย** — ว่าง+ยังไม่โฟกัส = ป้ายอยู่ในช่องเป็น placeholder / โฟกัสหรือพิมพ์แล้ว = ป้ายลอยขึ้นเกาะขอบบน
     (เจาะช่องว่างในเส้นขอบด้วยสีพื้นหลัง ให้ดูเหมือนลอยอยู่บนขอบจริง ๆ)
   - **ขอบเรืองแสงตอนโฟกัส** — วาด 2 ชั้นนอกกรอบ (`StrokeKind::Outside`) ให้ได้ความรู้สึกเป็นแสง ไม่ใช่เส้นหนา
   - **ตรวจสอบสด** — ✓ เขียว / ✕ แดงพร้อมคำอธิบาย (ไม่ใช่แค่ขึ้น error ตอนกดบันทึก)
   → ตัวช่วย: `ui::labeled_field()` + `ui::FieldStatus` ใน `src/ui.rs`
10. **ค่าที่ผู้ใช้พิมพ์ต้องอยู่นอกเฟรม** — ถ้าเก็บข้อความไว้ในตัวแปรชั่วคราวของ closure แต่ละเฟรม
   → ค่าจะถูกรีเซ็ตทุกเฟรม → **พิมพ์ไม่ติด** (เคยเกิดกับช่องแก้คีย์ลัด)
   → ต้องเก็บใน state ของแอป (`SettingsApp::edit_text`)

> ⚠️ **ข้อควรระวัง:** `egui::Frame::show()` สืบทอบ *เลย์เอาต์* ของ `Ui` พ่อแม่มา
> ถ้าอยู่ใน `Layout::bottom_up` เนื้อหาในเฟรมจะ (1) กลับด้านบน-ล่าง และ
> (2) ยืดเต็มพื้นที่ที่เหลือจนการ์ดสูงเท่าทั้งแถบ
> ทางแก้ที่ใช้จริง: จองพื้นที่เองด้วย `ui.allocate_ui_with_layout(size, Layout::top_down(..), ..)`
> แล้ววาดการ์ดในพื้นที่นั้น (เช่นแถบเมนูด้านซ้ายที่ต้องการกล่องสถานะติดขอบล่าง)

### 4.1 Cargo.toml (ของจริง)

```toml
[package]
name = "mackeys"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0.104"
arboard = { version = "3.7.0", features = ["image-data"] }
clap = { version = "4.6.7", features = ["derive"] }
dirs = "7.0.0"
eframe = { version = "0.37.0", default-features = false, features = ["wgpu", "default_fonts"] }
egui = "0.37.0"
image = { version = "0.25.10", features = ["png"] }
serde = { version = "1.0.229", features = ["derive"] }
thiserror = "2.0.21"
toml = "1.2.1"
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
windows = { version = "0.62.2", features = [
    "Win32_Foundation", "Win32_Graphics_Gdi", "Win32_Security",
    "Win32_Storage_FileSystem", "Win32_System_Console",
    "Win32_System_LibraryLoader", "Win32_System_Registry",
    "Win32_System_SystemInformation", "Win32_System_Threading",
    "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

> **ข้อสังเกต:** `windows` crate เวอร์ชัน 0.62 เปลี่ยนชื่อ type path และ signature หลายจุด
> (เช่น `GetKeyboardLayoutList(Option<&mut [HKL]>)`, `GetLocalTime()` ไม่มี argument,
> `BOOL`/`HWND`/`HBITMAP` อยู่คนละโมดูล) ต้อง grep จาก
> `~/.cargo/registry/src/.../windows-0.62.2/src/Windows/Win32/...` เสมอ อย่าเดาจากความจำ

---

## 5. Win32 ที่ต้องใช้ (พร้อมโค้ดตัวอย่าง)

### 5.1 การสลับภาษาอินพุต (หัวใจของงาน)

ไม่ควร simulate `Win+Space` เพราะมันจะเปิด overlay UI ของ Windows ให้เห็น แทนที่จะเปลี่ยนแบบเงียบ ๆ ให้ใช้ Win32 API ตรง ๆ:

```rust
// src/win/layout.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardLayoutList, HKL, LoadKeyboardLayoutW, KLF_ACTIVATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageW, GetGUIThreadInfo, GUITHREADINFO, WM_INPUTLANGCHANGEREQUEST,
};
use windows::Win32::Foundation::HWND;

const INPUTLANGCHANGE_FORWARD: usize = 0x0002;

/// สลับไป layout ถัดไปในรายการของระบบ
pub fn switch_to_next_layout() -> anyhow::Result<()> {
    unsafe {
        let fg = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        let tid = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(fg, None);

        // เลือกหน้าต่างที่ focus จริง (ไม่ใช่แค่ foreground)
        let mut gti = GUITHREADINFO::default();
        let _ = GetGUIThreadInfo(tid, &mut gti);
        let target = if gti.hwndFocus.0 != std::ptr::null_mut() { gti.hwndFocus } else { fg };

        // รายการ layout ที่ติดตั้งไว้
        let count = GetKeyboardLayoutList(0, None);
        if count <= 1 { return Ok(()); }          // มี layout เดียว ไม่ต้องทำอะไร
        let mut list: Vec<HKL> = vec![HKL::default(); count as usize];
        GetKeyboardLayoutList(count, Some(list.as_mut_ptr()));

        let cur = GetKeyboardLayout(tid);
        let idx = list.iter().position(|h| *h == cur).unwrap_or(0);
        let next = list[(idx + 1) % list.len()];

        // DefWindowProc ของแอปจะ activate layout ใหม่ให้
        PostMessageW(
            target,
            WM_INPUTLANGCHANGEREQUEST,
            windows::Win32::Foundation::WPARAM(INPUTLANGCHANGE_FORWARD),
            windows::Win32::Foundation::LPARAM(next.0 as isize),
        );
    }
    Ok(())
}

/// สลับไป layout ที่ระบุชื่อ เช่น "04090409" (en-US) หรือ "0409041D" (th-TH)
pub fn switch_to_layout_id(lang_id: u32) -> anyhow::Result<()> { /* LoadKeyboardLayoutW + PostMessage */ }
```

จุดที่ต้องระวัง:
- ถ้าเปิด "ใช้วิธีป้อนสำหรับแต่ละหน้าต่างแอป" ไว้ → ต้อง post ไปที่ focused window ของ thread นั้น (ใช้ `GetGUIThreadInfo` ตามตัวอย่าง)
- ถ้าอยากให้ layout ที่เลือกเป็นค่า default ของทั้งเครื่องด้วย → `SystemParametersInfoW(SPI_SETDEFAULTINPUTLANG, 0, hkl, SPIF_SENDWININICHANGE)`
- Thai/English ควรกำหนดเป็น **รายการที่เลือกได้** ไม่ใช่ "ไปตัวถัดไป" เพราะเครื่องอาจมี IME อื่นปนอยู่

### 5.2 RegisterHotKey + message-only window

```rust
// src/win/hotkey_register.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_NOREPEAT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, RegisterClassExW, WNDCLASSEXW, DefWindowProcW, HWND_MESSAGE, WINDOW_MESSAGE,
};

const WM_HOTKEY: u32 = 0x0312;

unsafe extern "system" fn wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if msg == WM_HOTKEY {
        let id = (wparam.0 & 0xFFFF) as u16;
        crate::engine::dispatch(id);   // ส่งต่อไปที่ action
        return windows::Win32::Foundation::LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// ผูก hotkey → คืน true ถ้าสำเร็จ, false ถ้ามีแอปอื่นจองไว้ก่อน
pub fn register(id: u16, mods: u32, vk: u32, hwnd: HWND) -> bool {
    unsafe { RegisterHotKey(Some(hwnd), id as i32, mods | MOD_NOREPEAT, vk).as_bool() }
}
```

ข้อควรระวัง:
- ต้องมี message loop (`GetMessageW` / `TranslateMessage` / `DispatchMessageW`) บน thread เดียวกับที่สร้าง window
- ใช้ `MOD_NOREPEAT` กันไม่ให้ยิงซ้ำตอนกดค้าง
- ต้อง `UnregisterHotKey` + `DestroyWindow` ตอนปิดโปรแกรม
- ต้องเก็บ delegate ของ `WndProc` ไว้ใน static ไม่งั้นจะถูก GC แล้ว crash

### 5.3 กินคีย์แบบ "Ctrl ค้าง + Tab" (เฟส 3 — hook)

```rust
// src/win/hook.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SetWindowsHookExW, CallNextHookEx, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
};

unsafe extern "system" fn hook(
    ncode: i32, wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if ncode == windows::Win32::UI::Input::KeyboardAndMouse::HC_ACTION as i32 {
        let ks = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if ks.flags & LLKHF_INJECTED == 0 {          // ข้าม input ที่ตัวเรา inject เอง
            if let Some(decision) = crate::engine::on_key_event(ks) {
                if decision.suppress {
                    return windows::Win32::Foundation::LRESULT(1);   // กินคีย์ ไม่ส่งต่อ
                }
            }
        }
    }
    CallNextHookEx(None, ncode, wparam, lparam)
}
```

จุดที่ต้องระวัง:
- ต้องแยก left/right modifier จาก `vkCode` (`VK_LCONTROL`/`VK_RCONTROL`, `VK_LMENU`/`VK_RMENU`)
- ต้องดู `flags & LLKHF_EXTENDED` เพื่อแยกปุ่มลูกศร/Insert/Delete กับ NumPad
- ต้องกรอง auto-repeat (`ks.time == last_time && ks.vkCode == last_vk`)
- callback ต้องทำงานเร็วมาก (<1ms) งานหนักต้องย้ายไป thread อื่น
- **UIPI**: inject เข้า window ที่รันสิทธิ์สูงกว่าเราไม่ได้

### 5.4 การ inject คีย์ (SendInput)

```rust
// src/win/inject.rs
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VK_LWIN, VK_SHIFT, VK_S,
};

pub fn win_shift_s() {
    unsafe {
        let inputs = [
            key_in(VK_LWIN.0 as u16, false),
            key_in(VK_SHIFT.0 as u16, false),
            key_in(VK_S.0 as u16, false),
            key_in(VK_S.0 as u16, true),
            key_in(VK_SHIFT.0 as u16, true),
            key_in(VK_LWIN.0 as u16, true),
        ];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
```

ใส่ `dwExtraInfo` เป็น magic number แล้วเช็คใน hook เพิ่มอีกชั้นกัน loop

### 5.5 จับภาพเต็มจอ + clipboard

```rust
// src/win/capture.rs
pub fn capture_fullscreen() -> anyhow::Result<()> {
    let mut screens = screenshots::Screen::all()?;
    let img = screens[0].capture_image()?;          // image::RgbaImage
    let path = pictures_dir().join(format!("Screenshot-{}.png", timestamp()));
    img.save(&path)?;                                // ต้อง feature "png"
    let mut clip = arboard::Clipboard::new()?;
    clip.set_image(arboard::ImageData::try_from(img)?)?;
    Ok(())
}
```

---

## 6. Threading model

| Thread | หน้าที่ |
|---|---|
| Main | message loop (`GetMessageW`) — จำเป็นสำหรับ `WM_HOTKEY` และ hook ถ้าใช้ |
| Worker | งานหนัก: จับภาพ, สลับ layout ที่ต้องโหลด, เขียน log → รับงานผ่าน `std::sync::mpsc` |

กฎเหล็ก: **hook callback ห้ามทำงานหนัก** — ส่งข้อมูลไป worker แล้วคืนค่าเร็วที่สุดเท่าที่ได้

---

## 7. Config (TOML)

```toml
# %APPDATA%\MacKeys\config.toml
[general]
enabled = true
log_level = "info"

# layout ที่อยากให้สลับไปมาระหว่างกัน (เรียงตามลำดับ)
[layouts]
cycle = ["en-US", "th-TH"]          # สลับไปตัวถัดไปในรายการนี้
default = "en-US"

[[bindings]]
keys = "alt+space"
action = { switch_layout = "cycle" }

[[bindings]]
keys = "ctrl+space"
action = { switch_layout = "cycle" }

[[bindings]]
keys = "ctrl+shift+3"
action = { screenshot = "full" }

[[bindings]]
keys = "ctrl+shift+4"
action = { screenshot = "region" }

[[bindings]]
keys = "ctrl+shift+5"
action = { screenshot = "panel" }

[[bindings]]
keys = "ctrl+tab"
action = { app_switcher = true }
```

รองรับ modifier: `ctrl`, `alt`, `shift`, `win`, `lctrl`/`rctrl`, `lalt`/`ralt`, `lshift`/`rshift`
ตัวอย่าง: `"ctrl+shift+3"`, `"lalt+space"`, `"ctrl+tab"`

---

## 8. UI (ของจริง)

**System tray** — คลิกซ้าย = เปิดหน้าต่างตั้งค่า, คลิกขวา = เมนูป็อปอัป:

- เมนู: สลับภาษาตอนนี้ / ตั้งค่า... / เปิดโฟลเดอร์ config / ออกจากโปรแกรม
- tooltip บนไอคอนเปลี่ยนเป็น "มี N คีย์ลัดที่ลงทะเบียนไม่ได้" เมื่อมี binding ล้มเหลว
- ปุ่มปิดหน้าต่าง GUI = ซ่อน ไม่ใช่ออกจากโปรแกรม (ออกได้ทางเมนู tray เท่านั้น)

**Settings GUI** — `egui` + `eframe` (wgpu / DirectX 12) อยู่ใน thread หลัก
- เปิด/ปิดทั้งโปรแกรม, แก้ไขคีย์ลัดและ action ของแต่ละ binding, เพิ่ม/ลบ binding
- แสดง layout ที่ติดตั้ง, เลือกภาษาเริ่มต้น, ลำดับการสลับ
- แสดงสถานะการลงทะเบียนของแต่ละคีย์ลัด (เขียว = พร้อม, แดง = ชนกับโปรแกรมอื่น)
- **โหลดฟอนต์ `C:\Windows\Fonts\tahoma.ttf`** เพิ่ม เพราะฟอนต์เริ่มต้นของ egui ไม่มีอักษรไทย

---

## 8.1 โหมด Mac: ให้ปุ่ม `Alt` ทำหน้าที่เป็น `⌘` (Command)

### ปัญหาที่ต้องแก้

ผู้ใช้ต้องการ "กดแบบ macOS" แต่บน Windows ปุ่ม ⌘ คือ `Ctrl` ซึ่งทำให้ `Alt` กับ `Ctrl`
ทำหน้าที่ซ้ำกันสองอัน (สับสน) ทางออกคือ **ทำให้ปุ่ม Alt ฝั่งซ้าย "เป็น" Ctrl ตัวเดียว**
→ มี Ctrl อันเดียวจริง ๆ และกดแบบ macOS ได้

### ทางเลือกที่พิจารณา

| ทางเลือก | ข้อดี | ข้อเสีย |
|---|---|---|
| **A. hook ในตัวแอป** (`WH_KEYBOARD_LL`) | ไม่ต้อง admin / ไม่ต้อง reboot / เปิด-ปิดได้จาก GUI | เป็น hook (ต้องเขียน state machine ให้ดี) |
| **B. Scancode Map ของ Windows** | เสถียรที่สุด ไม่มี hook เลย แอปสิทธิ์สูงก็ได้ Ctrl จริง | ต้อง admin + reboot 1 ครั้ง และเปลี่ยนทั้งเครื่อง |
| **C. ผูกคีย์รายตัว** | ปลอดภัยสุด | Alt ไม่ทำหน้าที่ Ctrl ในคีย์ลัดอื่น → ไม่ตอบโจทย์ |

**เลือก A เป็นค่าเริ่มต้นในตัวแอป + แนบ B เป็นสคริปต์** (`scripts/alt-as-cmd.ps1`)

### กลไกของ hook (ทางเลือก A)

```
กด Alt ซ้าย   → กลืน (return 1) + inject "Alt up" เพื่อล้าง async key state
กดปุ่มอื่น     → inject "Ctrl down" แล้วปล่อยปุ่มนั้นผ่าน → แอปเห็น Ctrl+X
ปล่อย Alt      → inject "Ctrl up"
```

**โครงสร้างโค้ด (แยก logic ออกจาก Win32 เพื่อให้เทสต์ได้)**

| ไฟล์ | หน้าที่ |
|---|---|
| `src/win/hook_state.rs` | state machine ล้วน ๆ (ไม่ติดต่อ Win32) — มี unit test ครบทุกสถานการณ์ |
| `src/win/hook.rs` | ตัวห่อ Win32 (hook callback + SendInput + watchdog) บาง ๆ |

state machine รับ `Event::KeyDown/KeyUp` + เวลา (มิลลิวินาที) แล้วคืน `Vec<Effect>`
(`CtrlDown` / `CtrlUp` / `AltUp`) → ทดสอบได้ครบทุกกรณีโดยไม่ต้องกดคีย์จริง

**จุดที่ต้องระวัง (เจอจากการทดสอบจริง):**

1. **ต้องล้าง async key state ของ Alt ทันที** — Windows อัปเดต async state
   *ก่อน* เรียก low-level hook ถ้าไม่ล้าง ระบบจะยัง "เห็น" ว่า Alt กดอยู่ →
   `RegisterHotKey` (ที่ต้องการ modifier ตรงเป๊ะ) จะไม่ยอมรับ `Ctrl+Space`
   เพราะกลายเป็น `Ctrl+Alt+Space` → แก้ด้วยการ inject `Alt up` (ใช้ `VK_LMENU`)
   > ข้อสำคัญ: `Alt up` ต้อง inject **ตอนกด Alt** (ไม่ใช่ตอนกดปุ่มที่สอง)
   > เพราะ hotkey matcher ดูสถานะ ณ เวลาที่ปุ่มที่สองถูกประมวลผล
   > ถ้า inject ช้ากว่านั้น ระบบจะเห็น `Ctrl+Alt+Space` และไม่ยอมรับ
2. **อินพุตที่ถูก inject ต้องไม่ถูกแปลงซ้ำ** — ใช้ `LLKHF_INJECTED` ตรวจ
   และใส่ marker ใน `dwExtraInfo` เพื่อแยกอินพุตของ MacKeys เองออกจากของโปรแกรมอื่น
3. **ปล่อย `Ctrl` ต้องรอให้ปุ่มอื่นค้างหมด** — ถ้าปล่อยเร็วเกินไป แอปจะเห็น
   `Ctrl` หลุดก่อนปุ่มอื่น → คอมโบไม่ตรง
   (ถ้าผู้ใช้ปล่อย Alt ก่อนปุ่มอื่น ต้องค้าง `Ctrl` ไว้จนกว่าปุ่มอื่นจะปล่อย)
4. **การ inject คีย์จะไปโดน hotkey ของตัวเอง** — เช่น inject `Alt+Tab`
   ไปโดน `Ctrl+Tab` ที่ลงทะเบียนไว้ → วนลูป (log เห็น "สลับหน้าต่าง" 3 ครั้ง)
   แก้ด้วยธง `SUPPRESS_HOTKEYS` ที่เปิดระหว่าง inject
5. **hook ต้องคืนค่าเร็ว** — ถ้าช้ากว่า `LowLevelHooksTimeout` (300 ms)
   Windows จะถอด hook ออกเองโดยไม่บอก
6. **ห้ามนับ "ปุ่มที่กดค้าง" ด้วยการนับ event** — auto-repeat ส่ง `keydown`
   ซ้ำหลายครั้ง แต่ `keyup` มีครั้งเดียว ถ้านับเป็นตัวเลข ตัวเลขจะไม่กลับเป็น 0
   → `Ctrl` ที่ inject ไว้ไม่ถูกปล่อย → **Ctrl ค้างทั้งระบบ**
   → อาการ: พิมพ์ไม่ได้ (ทุกตัวอักษรกลายเป็น `Ctrl+<ตัวอักษร>`) และ `Alt+Space` ใช้ไม่ได้
   (เพราะกลายเป็น `Ctrl+Alt+Space` ซึ่งไม่ตรงกับ hotkey ที่ลงทะเบียนไว้)
   แก้แล้วโดยเก็บสถานะเป็น **รายปุ่ม** (`held: [bool; 256]`) แทนการนับ event
7. **auto-repeat ของ Alt ต้องไม่ล้างรายการปุ่มที่กดค้าง** — ถ้าล้าง ตัวนับจะเพี้ยน
   ทำให้ `Ctrl` หลุดผิดจังหวะ (ปุ่มที่กดอยู่ยังไม่ปล่อย แต่ `Ctrl` ถูกปล่อยแล้ว)
8. **`keyup` ของ Alt อาจหายไป** (ระหว่าง UAC / secure desktop / hook ถูกถอด)
   → state ค้างว่า Alt ยังกดอยู่ → **ทุกปุ่มที่กดต่อไปกลายเป็น `Ctrl+<ปุ่ม>`**
   → พิมพ์ไม่ได้ + `Alt+Space` ใช้ไม่ได้ และแก้ได้ด้วยการกด Ctrl อีกครั้งเท่านั้น
   แก้แล้วโดยใช้ **heartbeat**: Windows ส่ง auto-repeat ของ `Alt` มาเป็น `keydown` ซ้ำ ๆ
   ตอนผู้ใช้กดค้าง → ถ้า heartbeat หยุดเกิน `ALT_STALE_MS` แปลว่า `keyup` หาย
   → ปล่อย `Ctrl` ทิ้ง + รอ heartbeat ยืนยันก่อนล้างสถานะ `Alt`
   (ถ้าไม่เคยเห็น auto-repeat เลย = ไดรเวอร์ไม่ repeat modifier → ใช้เวลาผ่อนยาวกว่า กัน false positive)
9. **โปรแกรมอาจตาย/ถูกปิดขณะผู้ใช้กด ⌘ ค้าง** → `Ctrl` ที่ inject ไว้ไม่มีโอกาสถูกปล่อย
   → ค้างถาวรจนกว่าจะกด Ctrl อีกครั้ง
   แก้แล้วโดย (ก) ปล่อย `Ctrl` ใน `uninstall()`/`set_enabled(false)` และ
   (ii) watchdog ทุก 200 ms ตรวจ `GetAsyncKeyState` ว่ามี `Ctrl`/`Alt` ค้างที่ไม่ได้มาจากผู้ใช้กด
   แล้วปล่อยให้เอง + เขียน log ไว้

**ทางออกฉุกเฉิน**: เมนู system tray → "รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)"
และคำสั่ง `mackeys diagnose-keys` (ดูว่าปุ่มไหนค้างอยู่ตอนนี้)
10. **ห้ามปล่อย `Alt` ของผู้ใช้ทิ้ง** — watchdog ต้องเชื่อ "ผู้ใช้กดอยู่จริง" ไม่ใช่แค่
    `GetAsyncKeyState` เพราะการ inject ทำให้สถานะ "ค้าง" ปลอม ๆ ได้
    และ `Alt` ขวา (AltGr) ของผู้ใช้ต้องไม่ถูกปล่อย → จำสถานะแยกซ้าย/ขวา
    (`is_phys_alt_down`) แล้วเทียบก่อนปล่อย
11. **`Alt+Tab` ต้องไม่ทิ้ง `Ctrl` ที่ inject ค้างไว้** — ถ้าไม่ปล่อยก่อน
    ระบบจะเห็น `Ctrl+Alt+Tab` → task switcher เข้าโหมดผิดแล้วค้าง
    แก้ด้วย `suspend_ctrl()` → ส่ง `Alt+Tab` → `resume_ctrl()`
    (ใช้ `VK_LMENU` แทน `VK_MENU` ตัวรวม เพราะตัวรวมไม่ล้างสถานะซ้าย/ขวา)
12. **ห้ามเขียนไฟล์ในเส้นทางอินพุต** — thread ที่ติดตั้ง hook คือ thread เดียวกับ message loop
    ถ้าเขียน log ลงดิสก์แล้วดิสก์ค้าง → thread ค้าง → hook ตายเงียบทั้งช่วง
    (อาการเดียวกับ "กดไม่เปลี่ยน" แต่โปรแกรมยังทำงานอยู่)
    แก้แล้วโดยย้ายการเขียนไฟล์ไป thread แยก (`mackeys-log`) ที่รับผ่านคิวแบบไม่ block
13. **hook อาจถูก Windows ถอดเอง** (ไม่แจ้ง) → ต้องตรวจและซ่อมเอง
    ใช้หลักฐาน "ระบบเห็นอินพุต (hotkey ยิง) แต่ hook ไม่เห็นเหตุการณ์เกิน 2 วินาที"
    → ติดตั้ง hook ใหม่ (`hook_is_dead` + `install()` ที่ถอดของเดิมออกก่อนเสมอ)
14. **เปลี่ยนภาษาต้องทำงานโดยไม่ต้องอยู่ในช่องพิมพ์** — `WM_INPUTLANGCHANGEREQUEST`
    ส่งให้ "หน้าต่างที่โฟกัส" เท่านั้น แอปที่ไม่รับข้อความนี้ (Explorer, เกม, แอปวาดเอง) จะไม่มีผล
    → ใช้ **`Win+Space`** ให้ Windows สลับเอง (กลไกของระบบ ทำงานทุกที่)
    นับจำนวนครั้งที่ต้องกดเพื่อไปถึงเป้าหมายตาม `[layouts].cycle` (`presses_to_reach`) แล้วตรวจผลจริง
15. **`⌘+Tab` ต้องทำเป็น app switcher แบบ macOS** — เดิมปล่อย Alt ทันที (80 ms)
    → switcher ปิดก่อนเวลา → ผู้ใช้กด Tab ต่อเพื่อเลือกหน้าต่างไม่ได้
    แก้โดย: hook กด `Alt` จริง (ผ่านไปให้ Windows) + ยิง `Tab` แล้ว **กดค้างไว้**
    จนกว่าผู้ใช้จะปล่อย `⌘` → ปล่อย `Alt` → commit
    • ต้องไม่ inject `Ctrl` ตอน `⌘+Tab` (จะกลายเป็น `Ctrl+Alt+Tab` → switcher เพี้ยน/ค้าง)
    • ต้อง **กลืน `Tab`/`Space` จริง** ไม่งั้นระบบเห็น 2 ครั้ง (ของจริง + ที่ inject)
    • อินพุตที่ต้อง "ผ่านถึงระบบจริง" ใช้ marker `INJECT_PASS_MARKER` (ต่างจาก `INJECT_MARKER` ที่ถูกกลืน)

### คีย์ลัดแบบ macOS ที่ Windows ไม่มี

| macOS | บน Windows | ทำให้โดย |
|---|---|---|
| `⌘Q` ปิดแอป | ไม่มี | `Ctrl+Q` → inject `Alt+F4` |
| `⌘H` ซ่อนแอป | ไม่มี | `Ctrl+H` → inject `Win+Down` (ย่อหน้าต่างที่ active) |
| `⌘\`` สลับหน้าต่างของแอปเดียวกัน | `Alt+\`` | `Ctrl+\`` → inject `Alt+\`` |
| `⌘Space` Spotlight | ชนกับสลับภาษา | ใช้ `Alt+Space` สลับภาษาแทน (ตามที่ผู้ใช้เลือก) |

> ส่วน `⌘C / ⌘V / ⌘Z / ⌘A / ⌘S / ⌘W / ⌘1..9` ไม่ต้องทำอะไรเพิ่ม เพราะ Windows
> ผูกกับ `Ctrl` อยู่แล้ว และโหมด Mac จะแปลง Alt → Ctrl ให้เอง

---

| M | ขอบเขต | สถานะ |
|---|---|---|
| **M0** | scaffold โปรเจกต์ Rust + ไล่ดูเวอร์ชัน crate ที่ resolve ได้ | ✅ เสร็จ |
| **M1** | โครงโปรเจกต์, config, CLI, single-instance, tray, logging | ✅ เสร็จ |
| **M2** | `RegisterHotKey` + `Alt+Space` / `Ctrl+Space` → สลับภาษา | ✅ เสร็จ (เหลือแค่ปลดล็อก `Alt+Space` จาก PowerToys) |
| **M3** | จับภาพ 3/4/5 (เต็มจอ + clipboard / region ผ่าน `Win+Shift+S` / เปิด Snipping Tool) | ✅ เสร็จ |
| **M4** | `Ctrl+Tab` inject `Alt+Tab` ผ่าน `SendInput` | ✅ เสร็จ |
| **M5** | Settings GUI (egui) | ✅ เสร็จ |
| **M6** | build release + ทดสอบจริงบนเครื่อง | ✅ เสร็จ |
| **M7** | README + อัปเดต DESIGN ให้ตรงกับโค้ดจริง | ✅ เสร็จ |
| **M8** | app switcher แบบ macOS (overlay พร้อมไอคอน) ต้องใช้ `WH_KEYBOARD_LL` | ⏸ ยังไม่ทำ — เลื่อนออกไปก่อน ผู้ใช้เลือกใช้ `Alt+Tab` ของ Windows |
| **M9** | installer (`cargo-wix`/NSIS) | ⏸ ยังไม่ทำ — ใช้ `install-autostart` + copy exe ไปก่อน |

---

## 10. แผนทดสอบ + ผลที่ได้จริง

1. **`list-layouts`** — พิมพ์ layout ที่ติดตั้ง + ตัวปัจจุบัน → ✅ เห็น `en-US (0409)` และ `th-TH (041E)`
2. **`diagnose`** — ลอง register ทุก binding ทีละตัว รายงานว่าตัวไหนถูกจองไปแล้ว
   → ✅ `Ctrl+Space`, `Ctrl+Shift+3/4/5`, `Ctrl+Tab` ผ่าน · `Alt+Space` ล้มเหลว (PowerToys Run จองไว้)
3. **ทดสอบด้วยมือแต่ละคีย์** — ผลจริง:

| คีย์ | ผล |
|---|---|
| `Ctrl+Space` | ✅ สลับภาษาจริง (log บันทึก `th-TH` / `en-US`) |
| `Ctrl+Shift+3` | ✅ บันทึก PNG 5120×1440 + คัดลอก clipboard |
| `Ctrl+Shift+4` | ✅ `SnippingTool` process เปิดขึ้นจริงหลัง inject `Win+Shift+S` |
| `Ctrl+Shift+5` | ✅ หน้าต่าง Snipping Tool เปิดจริง (`ms-screensketch:`) |
| `Ctrl+Tab` | ✅ log "สลับหน้าต่างแล้ว" + หน้าต่าง foreground เปลี่ยนจริง |
| เมนู tray คลิกขวา | ✅ เมนู 4 รายการขึ้นครบ, ทุกรายการทำงาน (รวม "สลับภาษาตอนนี้" ที่ log ยืนยันการสลับภาษา) |
| หน้าต่างตั้งค่า | ✅ เปิดได้ แสดงผลภาษาไทยครบ, binding ที่ชนแสดงเป็นสีแดงพร้อมเหตุผล |

> **หมายเหตุเรื่องการทดสอบอัตโนมัติ:** การส่งคีย์ด้วย AutoHotkey แบบ `Send "^+3"` จะถูก AHK
> แปลงเป็น unicode injection ซึ่ง **ไม่ทำให้ `RegisterHotKey` ทำงาน** ต้องใช้
> `Send "{Ctrl down}{Shift down}{3}{Shift up}{Ctrl up}"` เพื่อกดคีย์จริง (physical key)

---

## 11. ข้อจำกัดที่ต้องบอกผู้ใช้

| ข้อจำกัด | ผลกระทบ | ทางออก |
|---|---|---|
| `Alt+Space` คือ System Menu ของ Windows | ปุ่มนั้นใช้ไม่ได้ทั่วเครื่อง | ปิด binding ได้, หรือใช้ `Ctrl+Alt+Space` |
| `RegisterHotKey` first-come-first-served | ชนกับ PowerToys Run / โปรแกรมอื่น | ตรวจผลลงทะเบียน + แจ้งเตือน + เสนอค่าสำรอง |
| ยกระดับสิทธิ์ (UIPI) | inject เข้า window admin ไม่ได้ | ยอมรับข้อจำกัด |
| `Alt+Space` ในบางแอปมีความหมายเดิม (Word, IDE) | พฤติกรรมเปลี่ยน | ตั้งค่าเป็นรายโปรแกรมได้ในอนาคต |
| โหมด "ต่าง input method ต่อหน้าต่างแอป" | ต้อง post ถึง focused window | จัดการด้วย `GetGUIThreadInfo` แล้ว |
| `RegisterHotKey` กินคีย์ให้แอปที่ focus อยู่ | เช่น `Ctrl+Tab` ในเบราว์เซอร์จะไม่เปลี่ยนแท็บ | ตั้งปิด binding เป็นรายตัวได้ |
| การ inject `Alt+Tab` จะไม่เห็น overlay แบบ macOS | ประสบการณ์ต่างจาก macOS | เผยแพร่ M8 (ต้องใช้ `WH_KEYBOARD_LL`) |

---

## 12. คำถามเปิด — ตอบแล้ว (ตามที่ผู้ใช้เลือก)

1. **ขอบเขต v1** — ✅ **ทั้งชุด**: สลับภาษา + จับภาพ 3/4/5 + สลับแอป
2. **App switcher** — ✅ ส่งต่อ `Alt+Tab` ของ Windows ไปก่อน (เร็ว เสถียร)
3. **UI** — ✅ มีหน้าตั้งค่า GUI (egui) ตั้งแต่แรก
4. **`Alt+Space` ชนกับ System Menu** — ✅ override ทั้งระบบ (ปิดได้ใน config)
