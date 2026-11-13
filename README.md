# MacKeys

คีย์ลัดสไตล์ macOS สำหรับ Windows — เขียนด้วย Rust

[![Release v0.1.0](https://img.shields.io/badge/release-v0.1.0-blue)](https://github.com/0xstillb/mackeys/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

| macOS | บน Windows (MacKeys) | ทำงานแบบ |
|---|---|---|
| `Ctrl+Space` | `Ctrl+Space` / **`Alt+Space`** | สลับภาษาอินพุต (ไทย ↔ อังกฤษ) |
| `Cmd+Tab` | `Ctrl+Tab` | สลับหน้าต่าง (ส่งต่อ `Alt+Tab` ของ Windows) |
| `Cmd+Shift+3` | `Ctrl+Shift+3` | จับภาพทั้งจอ → บันทึกไฟล์ + คัดลอก clipboard |
| `Cmd+Shift+4` | `Ctrl+Shift+4` | จับภาพบริเวณ (Snipping Tool ของ Windows) |
| `Cmd+Shift+5` | `Ctrl+Shift+5` | เปิดหน้าต่าง Snipping Tool |

> ใน **โหมด Mac** (ปุ่ม `Alt` ซ้าย = `⌘`): `⌘+Space` สลับภาษาผ่าน `Win+Space`
> และ `⌘+Tab` เปิด task switcher แบบ macOS (กด `⌘` ค้าง + `Tab` ไปเรื่อย ๆ ได้)

> `Alt+Space` ตามปกติคือ System Menu ของหน้าต่าง — MacKeys จะ override ไปทั้งระบบ (ปิดได้ในไฟล์ config)

## โหมด Mac: ปุ่ม Alt = ⌘ (Command)

เปิดโหมดนี้ได้จากหน้าต่างตั้งค่า (สวิตช์ **"ให้ปุ่ม Alt ฝั่งซ้ายทำหน้าที่เป็น Ctrl"**) หรือในไฟล์ config:

```toml
[mac]
enabled = true
```

**ผลลัพธ์** — ปุ่ม `Alt` ฝั่งซ้ายจะทำหน้าที่เป็น `Ctrl` (คีย์ ⌘ ของ macOS) แบบเหมือนกดเมาส์:

| กด | ได้ผลเหมือน macOS | ทำงานโดย |
|---|---|---|
| `Alt+C` | ⌘C คัดลอก | Windows เอง (Ctrl+C) |
| `Alt+V` | ⌘V วาง | Windows เอง |
| `Alt+Z` / `Alt+Shift+Z` | ⌘Z undo / redo | Windows เอง |
| `Alt+A` | ⌘A เลือกทั้งหมด | Windows เอง |
| `Alt+S` | ⌘S บันทึก | Windows เอง |
| `Alt+W` | ⌘W ปิดแท็บ | Windows เอง |
| `Alt+1`…`Alt+9` | ⌘1…⌘9 สลับแท็บ | Windows เอง |
| **`Alt+Space`** | ⌘Space → **สลับภาษาอินพุต** | MacKeys (ส่ง `Win+Space` ให้ Windows สลับเอง) |
| **`Alt+Tab`** | ⌘Tab สลับแอป (กด ⌘ ค้าง + Tab ไปเรื่อย ๆ ได้) | MacKeys (จัดการเองทั้งหมด) |
| **`Alt+Shift+3/4/5`** | ⌘⇧3/4/5 จับภาพ | MacKeys |
| **`Alt+Q`** | ⌘Q ปิดแอป | MacKeys (ส่งต่อ `Alt+F4`) |
| **`Alt+H`** | ⌘H ซ่อนแอป | MacKeys (ส่งต่อ `Win+Down` = ย่อหน้าต่างที่ active) |
| **`Alt+`** | ⌘` สลับหน้าต่างของแอปเดียวกัน | MacKeys (ส่งต่อ `Alt+``) |

> **ไม่มี Ctrl สองอันแย่งกัน** — ปุ่ม Alt ฝั่งซ้ายคือ Ctrl ตัวเดียว ส่วน `Ctrl` ดั้งเดิมยังใช้ได้ตามปกติ
> ปุ่ม **Alt ฝั่งขวา** ยังเป็น Alt จริง (ไว้ใช้ `Alt+Tab` / `Alt+F4` / เมนูของ Office แบบเดิม)

### ⌘+Space เปลี่ยนภาษาทุกที่ (ไม่ต้องอยู่ในช่องพิมพ์)

เดิมใช้ `WM_INPUTLANGCHANGEREQUEST` ส่งให้ **หน้าต่างที่โฟกัสอยู่** เท่านั้น
→ ถ้าอยู่ในช่องพิมพ์ก็สลับได้ แต่ถ้าอยู่ที่อื่น (Explorer, เดสก์ท็อป, เกม, แอปที่วาดเอง)
ข้อความนี้จะถูกเมิน → **ไม่มีอะไรเกิดขึ้น**

ตอนนี้เปลี่ยนมาใช้ **`Win+Space`** ให้ Windows สลับภาษาเอง (กลไกของระบบ)
→ ทำงานทุกที่ ไม่ต้องอยู่ในช่องพิมพ์ และไม่ต้องพึ่งลำดับ layout ใน config

MacKeys นับจำนวนครั้งที่ต้องกด `Win+Space` เพื่อไปถึงภาษาเป้าหมายตาม `[layouts].cycle`
(ฟังก์ชัน `presses_to_reach`) แล้วตรวจผลจริงว่าเปลี่ยนจริงไหม
ถ้าไม่เปลี่ยน → ถอยไปใช้ `WM_INPUTLANGCHANGEREQUEST` เป็นทางสำรอง

### ⌘+Tab แบบ macOS (กด ⌘ ค้าง แล้ว Tab ไปเรื่อย ๆ)

เดิม `⌘+Tab` เปิด task switcher แล้ว **ปล่อย Alt ทันที** (80 ms)
→ task switcher ปิดก่อนเวลา → ผู้ใช้กด Tab ต่อเพื่อเลือกหน้าต่างไม่ได้

ตอนนี้ hook จัดการเองทั้งหมด:
1. กด `⌘+Tab` ครั้งแรก → กด `Alt` จริง (ผ่านไปให้ Windows) + ยิง `Tab` → switcher เปิดและ**ค้างไว้**
2. กด `Tab` ซ้ำ → เลื่อนตัวที่เลือก (Alt ที่เรากดไว้ยังค้างอยู่)
3. ปล่อย `⌘` → ปล่อย `Alt` → switcher ปิดและ commit ตัวที่เลือก

จุดสำคัญ: ตอน `⌘+Tab` **ไม่ inject `Ctrl`** (ถ้า inject จะกลายเป็น `Ctrl+Alt+Tab` → switcher เพี้ยน/ค้าง)
และ `Tab`/`Space` จริงต้องถูก **กลืน** ไม่งั้นระบบจะเห็น 2 ครั้ง (ของจริง + ที่ inject)

### วิธีทำงาน

- ใช้ low-level keyboard hook (`WH_KEYBOARD_LL`) ดักปุ่ม Alt ฝั่งซ้าย
  - กด Alt → กลืนไว้ และ "ล้าง" สถานะ Alt ในระบบ
  - กดปุ่มอื่นขณะ Alt ค้าง → inject `Ctrl` ลง แล้วปล่อยปุ่มนั้นผ่าน
    → แอปที่ได้รับเห็นเป็น `Ctrl+<ปุ่ม>` พอดี
  - ปล่อย Alt → ปล่อย `Ctrl`
- อินพุตที่โปรแกรมอื่น inject จะ **ไม่ถูกแปลง** (กันไม่ให้แอปอื่นโดนผลของโหมด Mac)
- ต้องล้างสถานะ Alt ก่อน ไม่งั้น Windows จะมองว่าเป็น `Ctrl+Alt+...`
  ทำให้ `RegisterHotKey` (ซึ่งต้องการ modifier ตรงเป๊ะ) ไม่ยอมรับคอมโบ

### ทางเลือกแบบไม่ใช้ hook (เสถียรกว่า)

ถ้าไม่อยากใช้ hook ให้แก้ที่ระดับไดรเวอร์แทน — แอปทุกตัว (รวมโปรแกรมสิทธิ์สูง) จะได้รับ Ctrl จริง ๆ
ไม่มี input lag และไม่ต้องเปิดโปรแกรมไว้:

```powershell
# ต้องรันด้วยสิทธิ์ Administrator แล้ว reboot 1 ครั้ง
powershell -ExecutionPolicy Bypass -File .\scripts\alt-as-cmd.ps1

# ย้อนกลับ
powershell -ExecutionPolicy Bypass -File .\scripts\alt-as-cmd.ps1 -Disable
```

> โหมด Mac ในตัวแอปกับสคริปต์นี้ **อย่าเปิดพร้อมกัน** — ถ้าเปิดสคริปต์แล้ว ให้ปิดโหมด Mac ในแอป

> **อย่าให้ AutoHotkey ทำงานพร้อมกับ MacKeys** — ถ้าเคยมีสคริปต์ AHK ที่ทำหน้าที่เดียวกัน
> (เช่น `langswitch.ahk` ที่ map `Ctrl+Tab` → `Alt+Tab`) ให้ **เอา shortcut ออกจาก Startup folder**
> ไม่งั้นทั้งสองโปรแกรมจะยิงคีย์ลัดพร้อมกัน (คีย์ลัดยิง 2 ครั้ง) และ
> AHK ยังใช้ keyboard hook ของตัวเอง → แย่ง input path กับ MacKeys → อินพุตค้าง

### ⌘+Shift+4 (จับภาพบริเวณ) ไม่ทำงาน

อาการ: กด `⌘+Shift+4` แล้วไม่มีอะไรเกิดขึ้น (Snipping Tool ไม่เปิด)

สาเหตุ: ตอนผู้ใช้กด `⌘` โหมด Mac จะ inject `Ctrl` ค้างไว้ (เพื่อให้ hotkey `Ctrl+Shift+4` ยิงได้)
ถ้ายังกด Ctrl อยู่ตอนส่ง `Win+Shift+S` → ระบบเห็นเป็น **`Ctrl+Win+Shift+S`** → Snipping Tool ไม่รู้จัก

แก้แล้ว: **ทุกการ inject คอมโบที่ "ไม่มี Ctrl ในตัวเอง" ต้องปล่อย Ctrl ที่ค้างอยู่ก่อนเสมอ**
(`inject::send_clean_combo`) แล้วค่อยกดคืนหลัง — ครอบคลุม `Win+Shift+S`, `Win+Space`, `Alt+F4`, `Alt+``

> กติกาเดียวกันนี้ใช้กับทุกคอมโบที่ MacKeys ส่งเอง ไม่ว่าจะมาจากทางไหน
> (เคยเป็นบั๊กซ้ำ ๆ: `⌘+Q` ไม่ปิดแอป, `⌘+Shift+4` ไม่เปิด Snipping Tool)
> มีเทสต์คุมไว้ที่ `win::inject::tests::combos_without_ctrl_need_clean_modifiers`

## การติดตั้ง

```powershell
cargo build --release
# ไฟล์โปรแกรมอยู่ที่ target\release\mackeys.exe
```

ก๊อป `mackeys.exe` ไปไว้ที่ไหนก็ได้ที่อยาก แล้วรัน — โปรแกรมจะไปอยู่ใน system tray
(ไอคอนคือกล่องสีน้ำเงินมนๆ ที่มีปุ่มคีย์บอร์ดสีขาว 3×3)

ตั้งให้รันพร้อม Windows:

```powershell
.\mackeys.exe install-autostart
```

## การใช้งานจาก system tray

| การกระทำ | ผลลัพธ์ |
|---|---|
| คลิกซ้ายที่ไอคอน | เปิดหน้าต่างตั้งค่า |
| คลิกขวาที่ไอคอน | เปิดเมนู: สลับภาษาตอนนี้ / ตั้งค่า... / เปิดโฟลเดอร์ config / **รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)** / ออกจากโปรแกรม |
| ปิดหน้าต่างตั้งค่า (กากบาท ✕) | ซ่อนหน้าต่าง (โปรแกรมยังทำงานอยู่ใน tray) |
| ออกจากโปรแกรม | เลือก "ออกจากโปรแกรม" ในเมนูคลิกขวา |

tooltip บนไอคอนจะขึ้นว่า "มี N คีย์ลัดที่ลงทะเบียนไม่ได้" ถ้ามีคอมโบนี้ชนกับโปรแกรมอื่น

### ถ้าพิมพ์ไม่ได้ / คีย์ลัดไม่ทำงาน

อาการ "พิมพ์ไม่ได้ ทุกตัวอักษรกลายเป็น Ctrl+<ตัวอักษร>" หรือ "`Alt+Space` ใช้ไม่ได้"
แปลว่า **มีปุ่ม modifier ค้างอยู่ในระบบ** (โหมด Mac จะแปลง `Alt` เป็น `Ctrl` แล้ว inject)

แก้ได้ 3 วิธี (เรียงจากง่ายไปยาก):

1. **คลิกขวาที่ไอคอนใน system tray → "รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)"** — แก้ได้ทันที
2. **กดและปล่อยปุ่ม `Ctrl` หนึ่งครั้ง** — Windows จะรีเซ็ตสถานะคีย์ให้เอง
3. `mackeys diagnose-keys` — ดูว่าปุ่มไหนค้างอยู่

โปรแกรมมีกลไกกันไว้ให้แล้ว: ตรวจทุก 200 ms ว่ามี `Ctrl`/`Alt` ค้างที่ไม่ได้มาจากผู้ใช้กด
แล้วปล่อยให้เอง พร้อมเขียน log ไว้ว่า `"พบ Ctrl ค้างในระบบ — ปล่อยให้อัตโนมัติ"`

> **หมายเหตุ (สำคัญ):** การปล่อยปุ่มที่ค้างต้องปล่อย **ทั้งซ้ายและขวา** รวมถึงตัวรวม
> (`VK_LCONTROL` + `VK_RCONTROL` + `VK_CONTROL`) — การปล่อยแค่ตัวรวม (`VK_CONTROL`)
> ไม่ล้างสถานะซ้าย/ขวา → ปุ่มยังค้างจริง → ผู้ใช้พิมพ์ไม่ได้
>
> **ข้อควรระวังเรื่อง AltGr:** ปุ่ม `Alt` ขวา (หรือ AltGr) ของผู้ใช้ **ห้ามถูกปล่อยทิ้ง**
> ระบบจึงจำสถานะ "ผู้ใช้กด Alt จริง" แยกซ้าย/ขวา (`is_phys_alt_down`)
> และ watchdog จะปล่อย Alt ให้เฉพาะตอนที่ผู้ใช้ **ไม่ได้กด** Alt อยู่จริงเท่านั้น

### ⌘+Tab (Alt+Tab) ค้างไม่ปล่อย

อาการเดิม: กด `Alt+Tab` แล้วหน้าต่างสลับขึ้นค้าง แม้ปล่อยปุ่มแล้ว

สาเหตุ: โหมด Mac inject `Ctrl` ค้างไว้ → พอส่ง `Alt+Tab` ระบบเห็นเป็น **`Ctrl+Alt+Tab`**
→ task switcher ของ Windows เข้าสู่โหมดผิดแล้วค้างอยู่

แก้แล้วโดย:
- ก่อนส่ง `Alt+Tab` → **ปล่อย `Ctrl` ที่ inject ค้างอยู่ออกก่อน** (`suspend_ctrl`)
- ส่ง `Alt+Tab` เสร็จ → กด `Ctrl` กลับคืน (`resume_ctrl`) เฉพาะกรณีที่ผู้ใช้ยังกด ⌘ อยู่
- ใช้ `VK_LMENU` (Alt ซ้าย) แทน `VK_MENU` (ตัวรวม) เพื่อไม่ให้สถานะ Alt ค้างในระบบ

### hook ถูก Windows ถอดเอง (อาการ: กด ⌘ แล้วไม่มีผล แต่ hotkey อื่นยังทำงาน)

Windows ถอด low-level keyboard hook ทิ้งเองโดยไม่แจ้ง ถ้า callback ช้าเกิน
`LowLevelHooksTimeout` (ค่าเริ่มต้น 300 ms) → โหมด Mac จะ "ตาย" เงียบ ๆ

MacKeys ตรวจจับเอง: ถ้า hotkey ยิง (แปลว่าผู้ใช้กดปุ่มจริง) แต่ hook ไม่เห็นเหตุการณ์เลย
นานกว่า 2 วินาที → ถือว่า hook ตาย → ติดตั้งใหม่ให้อัตโนมัติ
(ดูสถานะได้จาก log: `"hook ไม่ตอบสนอง ... ติดตั้ง hook ใหม่"`)

### ไม่มีการเขียนไฟล์ในเส้นทางอินพุต (กัน hook ตายเงียบ)

thread ที่ติดตั้ง hook คือ thread เดียวกับที่วน message loop → **ถ้างานบน thread นี้ค้าง
hook จะหยุดทำงานทันที** (ผู้ใช้กดอะไรก็ไม่มีผล แต่โปรแกรมยังทำงานอยู่)

เดิมมีจุดเสี่ยง: การเขียน log ลงไฟล์ทำบน thread นี้ → ถ้าดิสก์ค้าง (antivirus, OneDrive, defrag)
thread จะค้างตาม → อาการเหมือน "กดไม่เปลี่ยน" ทั้งช่วง

แก้แล้ว: ทุกอย่างที่ `tracing` บันทึกถูกส่งผ่านคิวไปยัง **thread แยก (`mackeys-log`)**
เป็นคนเขียนไฟล์จริง → thread หลักไม่เคยแตะดิสก์ (คิวเต็มก็ทิ้ง log ทิ้ง ไม่ block)

## หน้าต่างตั้งค่า (UI)

หน้าต่างเป็นโทนมืดสมัย (Fluent-like) ขนาด 900×640 (ต่ำสุด 760×520) แบ่งเป็น 3 ส่วน:

| ส่วน | รายละเอียด |
|---|---|
| แถบบน (62 px) | โลโก้ + ชื่อแอป + ปุ่ม "บันทึก" (เปิดใช้เมื่อมีการแก้ไข) |
| แถบเมนูซ้าย (224 px) | 4 หน้า — ภาพรวม / คีย์ลัด / ภาษาอินพุต / ระบบ พร้อมกล่องสรุปสถานะด้านล่าง |
| พื้นที่หลัก | เนื้อหาแบบเลื่อนได้ ประกอบด้วยการ์ดมน |

ชิ้นส่วนที่ใช้ซ้ำทั้งหมดอยู่ใน `src/ui.rs` (design system): โทนสี, การ์ด, สวิตช์แบบ iOS/Windows 11,
คีย์แคปสไตล์ macOS, ป้ายสถานะ (pill), ปุ่ม, และโลโก้ที่วาดด้วย painter

**ไอคอนในเมนูวาดเองด้วย painter** (แผงภาพรวม / คีย์บอร์ด / โลก / สไลเดอร์) ไม่ใช้ตัวอักษรสัญลักษณ์
เพราะฟอนต์ที่มีใน Windows ไม่ครอบคลุมทุกตัว (บางตัวออกมาเป็นกล่อง "tofu")

**ฟอนต์:** `Tahoma` (ไทย + ละติน) + `Segoe UI Symbol` (สัญลักษณ์ ⌘) + `Segoe UI Emoji`
เรียงเป็นสายอักษรเดียวกันเพื่อให้ egui fallback ได้เอง
(มีเทสต์ `gui::tests::font_has_command_symbol` คุมไว้)

> **สำหรับผู้พัฒนา:** ตั้ง `MACKEYS_SHOW_UI=1` เพื่อเปิดหน้าต่างทันทีตอนสตาร์ท
> และ `MACKEYS_PAGE=shortcuts|languages|system` เพื่อเปิดที่หน้าที่ต้องการตรวจ
> ถ่ายภาพหน้าต่างได้ด้วย `scripts/shot.ps1 -Out <ไฟล์.png>`

### ตัววาดกราฟิก: ใช้ DirectX 12 (wgpu) ไม่ใช่ OpenGL

`eframe` ตั้งค่าไว้แบบ `default-features = false, features = ["wgpu", "default_fonts"]`
→ ใช้ **wgpu (DirectX 12 / Vulkan)** วาดหน้าต่าง ไม่ต้องพึ่ง OpenGL

เหตุผล (เจอจริงบนเครื่องนี้): เครื่องไม่มี **OpenGL ICD ของผู้ผลิตการ์ด** ติดตั้ง
(`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\OpenGLDrivers` ว่าง และไม่มี `amdocl64.dll` ใน `System32`)
→ โปรแกรมที่ใช้ OpenGL จะได้แค่ **GL 1.1 ของ Microsoft** (software)
→ egui ต้องการ GL 2.0+ → **หน้าต่างขาวโพลน วาดอะไรไม่ขึ้น** (แต่โปรแกรมยังทำงานปกติ)

อาการที่เจอ: เปิดหน้าต่างตั้งค่าแล้วเป็นสีขาวเปล่า ขณะที่ tray icon และคีย์ลัดยังทำงานปกติ
วิธีตรวจ: `Get-Process <แอป> | % Modules | ? ModuleName -match 'amdocl|opengl'`
ถ้าเจอแต่ `OPENGL32.dll` (ไม่มี `amdocl64.dll`) = ไม่มี ICD → ต้องใช้ wgpu

> หมายเหตุ: ถ้าอยากให้เกม/โปรแกรมอื่นที่ใช้ OpenGL กลับมาทำงานปกติ
> ต้องซ่อม OpenGL ICD ของ driver ที่ระดับระบบ (ต้องใช้สิทธิ์ Administrator) — เรื่องนี้แยกจาก MacKeys

### ดีไซน์หน้าต่าง: สเปกเลย์เอาต์ + ช่องกรอกสไตล์ Text Field

อ้างอิงดีไซน์จากรีล **"Text Field Part 1"** ของ `@ui_motion_prompts` (Instagram)
→ เอาหลักการของช่องกรอกมาใช้ 3 อย่าง: **ป้ายลอย · ขอบเรืองแสงตอนโฟกัส · ตรวจสอบสด**

**1) สเปกเลย์เอาต์ (`ui::layout`)** — ย้ายตัวเลขการจัดวางมาเป็นค่าคงที่กลาง เพื่อให้ทุกหน้าเรียงกันตรงกัน

| ค่า | ความหมาย |
|---|---|
| `NAV_WIDTH` = 224 | ความกว้างแถบเมนูด้านซ้าย |
| `NAV_PAD_X` = 12 | ระยะขอบแถบเมนู → รายการที่เลือกไม่ชิดเส้นแบ่ง |
| `CONTENT_PAD_X` = 24 | ระยะขอบพื้นที่เนื้อหา → หัวข้อหน้าไม่ชิดแถบเมนู |
| `CARD_GAP_X` = 0 | การ์ดไม่มีระยะเพิ่ม → **ขอบซ้ายของการ์ดตรงกับหัวข้อหน้าพอดี** |

เทสต์ที่คุมไว้: `ui::tests::heading_and_cards_share_the_same_left_edge`,
`ui::tests::nav_item_keeps_padding_on_both_sides`, `ui::tests::layout_tokens_follow_the_spacing_scale`

**2) ช่องกรอก (`ui::labeled_field`)** — ใช้กับแผงแก้ไขคีย์ลัด

- **ป้ายลอย** — ตอนช่องว่างและยังไม่โฟกัส ป้ายอยู่ในช่องเป็น placeholder
  พอโฟกัสหรือพิมพ์ ป้ายจะลอยขึ้นมาเกาะขอบบน (เจาะช่องว่างในเส้นขอบให้ดูเหมือนลอยจริง ๆ)
- **ขอบเรืองแสง** — ตอนโฟกัส ขอบจะเรืองแสงสีหลัก 2 ชั้น (วาดนอกกรอบ ไม่ทับเส้นขอบ)
- **ตรวจสอบสด** — ตรวจคีย์ลัดทุกครั้งที่พิมพ์ → ✓ เขียว / ✕ แดงพร้อมคำอธิบาย
  (เทสต์: `gui::tests::shortcut_status_accepts_valid_combo`, `gui::tests::shortcut_status_explains_invalid_combo`)
- แผงแก้ไขคีย์ลัดย้ายไปอยู่แถวของตัวเอง **ใต้** แถวคีย์ลัด (ไม่บีบจนล้นคอลัมน์)
- ข้อความที่พิมพ์เก็บใน `SettingsApp::edit_text` (ข้ามเฟรม) — เดิมเก็บในตัวแปรชั่วคราว
  → ข้อความถูกรีเซ็ตทุกเฟรม (พิมพ์ไม่ติด) → แก้แล้ว

> **สำหรับผู้พัฒนา:** ตั้ง `MACKEYS_EDIT=<เลขแถว>` เพื่อเปิดแผงแก้ไขคีย์ลัดทันทีตอนสตาร์ท (ใช้ตอนถ่ายภาพหน้าจอ)


## การทดสอบ

```powershell
cargo test --release
```

ตอนนี้มี **41 unit test** ผ่านหมด แบ่งเป็น 5 กลุ่ม:

| กลุ่ม | ครอบคลุม |
|---|---|
| `win::hook_state::tests` (13) | state machine ของโหมด Mac — แปลง Alt เป็น Ctrl, ไม่ล้างปุ่มที่ค้างตอน auto-repeat, ตรวจ keyup ของ Alt ที่หายไป, watchdog ปล่อย Ctrl, `reset` คืนสถานะ, จำสถานะ Alt ซ้าย/ขวา (ไม่ปล่อย AltGr ของผู้ใช้) |
| `win::hook::tests` (2) | การอ่านค่า `MACKEYS_TEST_INJECTED` (ต้องอ่านค่าจริง — `=0` ต้องแปลว่าปิด) + ตรวจจับ hook ตายจากหลักฐาน "ระบบเห็นอินพุตแต่ hook ไม่เห็น" |
| `gui::tests` (7) | ฟอนต์มีสัญลักษณ์ ⌘ / อักษรไทย, สายอักษรสำรอง, ป้ายข้อความที่อ่านง่าย + ตรวจคีย์ลัดแบบสด (ถูก/ผิด) |
| `ui::tests` (8) | ระบบเลย์เอาต์: ค่าทุกตัวเป็นพหุลของ 4, รายการเมนูมีระยะห่างเท่ากันสองข้าง, หัวข้อหน้ากับการ์ดตรงขอบซ้ายกัน, แปลง `ctrl` → `alt` ในโหมด Mac, จุด 3 จุดในโลโก้ไม่ซ้อนกัน |
| `win::inject::tests` + `win::layout::tests` (5) | คอมโบที่ส่งเองต้องปล่อย Ctrl ที่ค้างก่อน + นับจำนวนครั้งที่ต้องกดเพื่อเลื่อนภาษา |

`hook_state.rs` แยก logic ออกจาก Win32 ทั้งหมด (ไม่มี `unsafe` ไม่มีการเรียก API)
จึงทดสอบสถานการณ์ยาก ๆ ได้ เช่น "ผู้ใช้กด Alt ค้างแล้วปล่อย keyup หายไป" โดยไม่ต้องเดินเหตุการณ์จริง

## คำสั่ง

```powershell
mackeys                      # รันโหมดปกติ (tray + hotkey)
mackeys diagnose             # ตรวจว่าคีย์ลัดไหนใช้ได้/ชนกับโปรแกรมอื่น
mackeys diagnose-keys        # แสดงว่าปุ่ม modifier ไหนค้างอยู่ตอนนี้
mackeys list-layouts         # แสดง keyboard layout ที่ติดตั้ง
mackeys install-autostart    # ให้รันพร้อม Windows
mackeys uninstall-autostart  # ยกเลิกการรันพร้อม Windows
mackeys open-config-dir      # เปิดโฟลเดอร์ config
mackeys capture              # ทดสอบจับภาพทั้งจอ (บันทึกไฟล์ + clipboard) แล้วออก
mackeys icon-preview         # เขียนภาพไอคอนลง %TEMP%\mackeys-icon.png
mackeys --config <PATH>      # ใช้ไฟล์ config ที่ระบุ
```

## การตั้งค่า

คลิกขวาที่ไอคอนใน system tray → **ตั้งค่า...** เพื่อเปิดหน้าต่าง GUI
หรือแก้ไฟล์ `%APPDATA%\MacKeys\config.toml` โดยตรง

```toml
[general]
enabled = true
log_level = "info"

[layouts]
# ภาษาที่ใช้สลับไปมา เรียงตามลำดับ
cycle = ["en-US", "th-TH"]
default = "en-US"

[[bindings]]
keys = "alt+space"
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
action = "app_switcher"

[[bindings]]
keys = "ctrl+h"
action = { send_keys = "win+down" }
```

**รูปแบบ `action`**
- `{ switch_layout = "cycle" }` — สลับตามรายการใน `[layouts].cycle`
- `{ switch_layout = "next" }` — สลับไป layout ถัดไปที่ติดตั้งอยู่
- `{ switch_layout = "th-TH" }` — สลับไปภาษาที่ระบุ (`en-US`, `th-TH`, หรือรหัส hex เช่น `041d`)
- `{ screenshot = "full" | "region" | "panel" }`
- `"app_switcher"` — สลับหน้าต่าง
- `{ send_keys = "alt+f4" }` — ส่งชุดคีย์ไปยังหน้าต่างที่ focus อยู่ (ใช้ทำ ⌘Q / ⌘` ที่ Windows ไม่มี)

**รูปแบบ `keys`** — modifier (`ctrl` `alt` `shift` `win`) คั่นด้วย `+` แล้วตามด้วยชื่อปุ่ม
เช่น `alt+space`, `ctrl+shift+3`, `ctrl+tab`, `lalt+space`

## วิธีทำงานภายใน

- **จับคีย์** ด้วย `RegisterHotKey` (global hotkey ของ Win32) — ไม่ใช้ low-level hook
  จึงไม่มีความเสี่ยงทำให้คีย์บอร์ดทั้งระบบค้าง และไม่กิน CPU
- **สลับภาษา** เรียก `WM_INPUTLANGCHANGEREQUEST` ตรงกับหน้าต่างที่ focus อยู่
  (ไม่ยิง `Win+Space` ปลอม ๆ จึงไม่มี overlay ของ Windows โผล่ขึ้นมากวนตา)
- **จับภาพ** ใช้ GDI (`BitBlt`) เขียนเป็น PNG เก็บใน `Pictures\Screenshots` และคัดลอกลง clipboard
  (ส่งเป็น raw RGBA เข้า `arboard` — ถ้าส่ง PNG bytes ตรง ๆ จะทำให้ process crash)
- **จับภาพบริเวณ** ส่งต่อ `Win+Shift+S` ด้วย `SendInput` (ให้ Snipping Tool ของ Windows ทำงาน)
- **เปิด Snipping Tool** เรียก `ms-screensketch:` ผ่าน `ShellExecuteW`
- **สลับหน้าต่าง** inject `Alt+Tab` ด้วย `SendInput` (กด Alt ค้าง 80 ms แล้วปล่อย)
- **ไอคอน tray** สร้างเองทีละพิกเซลเป็น DIB 32bpp + mask 1bpp แล้ว `CreateIconIndirect`
  (ไม่ต้องพึ่งไฟล์ `.ico`)

### โครงสร้าง process

โปรแกรมรันใน process เดียว แต่แยก 3 thread:

| Thread | หน้าที่ |
|---|---|
| main | eframe — หน้าต่างตั้งค่า (ซ่อนไว้ตอนเริ่ม ถ้ามีแต่ tray) |
| win | message loop + system tray + `RegisterHotKey` |
| worker | ทำ action (สลับภาษา / จับภาพ) — ครอบด้วย `catch_unwind` เพื่อไม่ให้ลากทั้งโปรแกรมลง |

> ข้อควรระวัง: `TrackPopupMenu` เป็น modal loop ที่ pump ข้อความกลับเข้า window procedure
> ตอนเรียกต้อง **ปล่อย borrow ของ state ก่อน** ไม่งั้นจะเจอ `RefCell already borrowed`
> แล้วพังทั้งโปรแกรม (wnd_proc เป็น `extern "system"` → abort ทันที)

## โครงสร้างโค้ด

```
src/
├─ main.rs        entry point + คำสั่ง CLI
├─ cli.rs         clap definitions
├─ config.rs      โครงสร้าง config (TOML) + โหลด/บันทึก
├─ action.rs      Action enum (สลับภาษา / จับภาพ / สลับหน้าต่าง / ส่งคีย์)
├─ hotkey.rs      แปลง "alt+space" → virtual key + modifier
├─ engine.rs      ตัดสินใจว่าจะทำอะไรเมื่อกดคีย์
├─ runner.rs      win thread: window + message loop + ลงทะเบียน hotkey
├─ tray.rs        ไอคอน system tray + เมนู
├─ ui.rs          design system ของหน้าต่าง (โทนสี/การ์ด/สวิตช์/คีย์แคป/ไอคอนเวกเตอร์)
├─ gui.rs         หน้าต่างตั้งค่า (egui) — 4 หน้า: ภาพรวม / คีย์ลัด / ภาษาอินพุต / ระบบ
├─ autostart.rs   ตั้ง/ยกเลิกการรันพร้อม Windows (HKCU\..\Run)
├─ logging.rs     log ลง %LOCALAPPDATA%\MacKeys\logs
├─ singleton.rs   กันรันซ้ำ (named mutex)
└─ win/
   ├─ mod.rs
   ├─ hotkey_register.rs  RegisterHotKey / UnregisterHotKey
   ├─ layout.rs           จัดการ input language
   ├─ inject.rs           SendInput (Alt+Tab, Win+Shift+S, ส่งคีย์ตามชื่อ)
   ├─ capture.rs          จับภาพหน้าจอ + clipboard
   ├─ hook.rs             โหมด Mac: แปลงปุ่ม Alt ให้เป็น Ctrl (WH_KEYBOARD_LL)
   ├─ hook_state.rs       state machine ของ hook (ล้วน ๆ ไม่ติดต่อ Win32 → เทสต์ได้)
   ├─ icon.rs             สร้างไอคอน tray ตอนรัน
   └─ util.rs             ตัวช่วย Win32 ทั่วไป

scripts/
└─ alt-as-cmd.ps1        ทางเลือกแบบไม่ใช้ hook: แก้ Scancode Map ของ Windows
```

## ไฟล์ที่โปรแกรมสร้าง

| ตำแหน่ง | ไฟล์ |
|---|---|
| `%APPDATA%\MacKeys\config.toml` | config |
| `%LOCALAPPDATA%\MacKeys\logs\mackeys.log` | log |
| `Pictures\Screenshots\*.png` | ภาพที่จับจาก `Ctrl+Shift+3` |

## ผลการทดสอบจริงบนเครื่อง (Windows 11 26H2)

| รายการ | ผล |
|---|---|
| `Ctrl+Space` สลับภาษา | ✅ en-US ↔ th-TH (log ยืนยันทุกครั้ง) |
| `Ctrl+Shift+3` จับภาพทั้งจอ | ✅ ได้ PNG 5120×1440 + clipboard |
| `Ctrl+Shift+4` จับภาพบริเวณ | ✅ ส่งต่อ `Win+Shift+S` → Snipping Tool เปิดจริง |
| `Ctrl+Shift+5` เปิด Snipping Tool | ✅ หน้าต่าง Snipping Tool เปิดจริง |
| `Ctrl+Tab` สลับหน้าต่าง | ✅ ส่งต่อ `Alt+Tab` (หน้าต่าง focus เปลี่ยนจริง) |
| เมนู tray (คลิกขวา) | ✅ ครบ 4 รายการ และทุกรายการทำงาน |
| หน้าต่างตั้งค่า | ✅ เปิดได้ ภาษาไทยแสดงผลครบ + สัญลักษณ์ ⌘ แสดงผล (Tahoma → Segoe UI Symbol) |
| `Alt+Space` สลับภาษา | ✅ **ลงทะเบียนได้แล้ว** หลังปิด PowerToys Run (ที่เคยจองคอมโบนี้นี้ไว้) |
| โหมด Mac (Alt = ⌘) | ✅ ทำงาน + มี watchdog ปล่อยปุ่มค้างให้อัตโนมัติ |
| `⌘+Space` สลับภาษา (Win+Space) | ✅ เปลี่ยน en-US → th-TH ได้โดยไม่ต้องอยู่ในช่องพิมพ์ |
| `⌘+Tab` แบบ macOS | ✅ กด ⌘ ค้าง + Tab ไปเรื่อย ๆ ได้ (Alt ถูกกดค้างจริง) แล้วปล่อย ⌘ → commit |
| `⌘+Q` ปิดแอป | ✅ ส่ง `Alt+F4` แล้วแอปที่โฟกัสถูกปิดจริง (ปล่อย Ctrl ที่ค้างก่อนส่ง → ไม่กลายเป็น `Ctrl+Alt+F4`) |
| หน้าต่างตั้งค่า (4 หน้า) | ✅ วาดครบทุกหน้า (wgpu / DirectX 12) จัดวางตรงกัน ไม่มีส่วนไหนโดนตัด |
| `⌘+Shift+4` จับภาพบริเวณ | ✅ Snipping Tool เปิดแบบเลือกบริเวณได้จริง (ปล่อย Ctrl ที่ค้างก่อนส่ง `Win+Shift+S`) |
| `⌘+Shift+5` เปิด Snipping Tool | ✅ เปิดหน้าต่าง Snipping Tool ได้จริง |

> เคยมีอาการ "พิมพ์ไม่ได้" เพราะ hook เขียน log ทีละคีย์ใน hot path
> แก้แล้วโดยย้าย I/O ออกจาก hook callback (บัฟเฟอร์ในหน่วยความจำ + flush ใน timer)
> ตอนนี้ trace log มี `took=...us` บอกเวลาที่ hook ใช้จริง

## ข้อจำกัด

- `Alt+Space` จะถูก override ทั้งระบบ → System Menu ของหน้าต่างใช้ไม่ได้ (ปิด binding ได้)
- `RegisterHotKey` เป็น first-come-first-served → ถ้ามีโปรแกรมอื่นจองคอมโบนี้ไว้ก่อน
  (เช่น **PowerToys Run** ที่ใช้ `Alt+Space` เป็นค่าเริ่มต้น) คีย์ลัดนั้นจะใช้ไม่ได้
  ดูผลการตรวจได้ด้วย `mackeys diagnose`
  วิธีแก้: เปิด PowerToys → PowerToys Run → เปลี่ยนคีย์ลัด `open_powerlauncher`
  (ไฟล์ `%LOCALAPPDATA%\Microsoft\PowerToys\PowerToys Run\settings.json`)
  — เครื่องนี้ปิด PowerToys ไปแล้ว จึงใช้ `Alt+Space` ได้ตามปกติ
- โปรแกรมที่รันด้วยสิทธิ์สูงกว่า (เช่น Task Manager แบบ admin) จะไม่รับ input ที่ inject เข้าไป (UIPI)
- การสลับภาษาใช้ `Win+Space` → Windows สลับให้เองทั้งเครื่อง (ถ้าอยากบังคับว่าให้ไปภาษาใดภาษาหนึ่งตาม config ต้องแก้ที่ `win/layout.rs`)
- เครื่องที่ไม่มี OpenGL ICD ของผู้ผลิตการ์ด → โปรแกรมยังทำงานครบทุกอย่าง
  แต่โปรแกรมอื่นที่ใช้ OpenGL จะเห็นหน้าต่างขาว (MacKeys เปลี่ยนไปใช้ wgpu/DirectX 12 แล้ว จึงไม่มีปัญหา)

## สัญญาอนุญาต (License)

MIT — ดูรายละเอียดในไฟล์ [LICENSE](LICENSE)

ใช้ ทำซ้ำ แก้ไข และแจกต่อได้ รวมถึงใช้เชิงพาณิชย์
ต้องเก็บข้อความลิขสิทธิ์และข้อความอนุญาตไว้ในสำเนาหรือส่วนสำคัญของซอฟต์แวร์